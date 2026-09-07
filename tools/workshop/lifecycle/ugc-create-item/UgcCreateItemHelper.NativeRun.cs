// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

// This is the deliberately separate mutating gate for one new Workshop item.
// It creates an empty item only. Upload and update remain SteamCMD operations
// guarded by tools/workshop/lifecycle/workshop_operator.

internal static partial class Program
{
    private static Result RunNative(Options options, Result result, PackageSnapshot package)
    {
        nint library = 0;
        nint errorBuffer = 0;
        nint callbackBuffer = 0;
        bool initialized = false;
        Journal? journal = null;
        try
        {
            library = NativeLibrary.Load(options.LibraryPath);
            result.NativeLibraryLoaded = true;
            nint initAddress = RequiredExport(library, "SteamAPI_InitFlat");
            result.LoadedLibraryPath = LoadedModulePath(initAddress);
            result.LoadedLibraryMatches = result.LoadedLibraryPath is not null
                && PathsEqual(result.LoadedLibraryPath, options.ResolvedLibraryPath);
            if (!result.LoadedLibraryMatches)
            {
                result.ErrorType = "LoadedLibraryIdentityMismatch";
                return result;
            }

            nint shutdownAddress = RequiredExport(library, "SteamAPI_Shutdown");
            nint appsGetterAddress = RequiredExport(library, "SteamAPI_SteamApps_v008");
            nint utilsGetterAddress = RequiredExport(library, "SteamAPI_SteamUtils_v010");
            nint userGetterAddress = RequiredExport(library, "SteamAPI_SteamUser_v023");
            nint ugcGetterAddress = RequiredExport(library, "SteamAPI_SteamUGC_v020");
            nint getAppIdAddress = RequiredExport(library, "SteamAPI_ISteamUtils_GetAppID");
            nint loggedOnAddress = RequiredExport(library, "SteamAPI_ISteamUser_BLoggedOn");
            nint subscribedAddress = RequiredExport(library, "SteamAPI_ISteamApps_BIsSubscribedApp");
            nint createItemAddress = RequiredExport(library, "SteamAPI_ISteamUGC_CreateItem");
            nint completedAddress = RequiredExport(library, "SteamAPI_ISteamUtils_IsAPICallCompleted");
            nint resultAddress = RequiredExport(library, "SteamAPI_ISteamUtils_GetAPICallResult");
            nint runCallbacksAddress = RequiredExport(library, "SteamAPI_RunCallbacks");

            errorBuffer = Marshal.AllocHGlobal(ErrorBufferCapacity);
            Marshal.WriteByte(errorBuffer, 0, 0);
            result.InitAttempted = true;
            result.InitResult = Export<InitFlatDelegate>(initAddress)(errorBuffer);
            if (result.InitResult != 0)
            {
                (result.InitFailureCategory, result.InitFailureDiagnosticPresent) =
                    ClassifyInitFailure(errorBuffer);
                result.ErrorType = "SteamInitFailed";
                return result;
            }

            initialized = true;
            nint apps = Export<InterfaceGetter>(appsGetterAddress)();
            nint utils = Export<InterfaceGetter>(utilsGetterAddress)();
            nint user = Export<InterfaceGetter>(userGetterAddress)();
            nint ugc = Export<InterfaceGetter>(ugcGetterAddress)();
            result.AppsInterfaceAvailable = apps != 0;
            result.UtilsInterfaceAvailable = utils != 0;
            result.UserInterfaceAvailable = user != 0;
            result.UgcInterfaceAvailable = ugc != 0;
            if (apps == 0 || utils == 0 || user == 0 || ugc == 0)
            {
                result.ErrorType = "RequiredInterfaceUnavailable";
                return result;
            }

            result.AppId = Export<GetAppIdDelegate>(getAppIdAddress)(utils);
            result.AppIdMatches = result.AppId == options.ExpectedAppId;
            if (!result.AppIdMatches)
            {
                result.ErrorType = "AppIdMismatchBeforeCreateItem";
                return result;
            }

            result.LoggedOn = Export<ByteResult>(loggedOnAddress)(user) != 0;
            result.Subscribed = Export<SubscribedResult>(subscribedAddress)(apps, options.ExpectedAppId) != 0;
            if (!result.LoggedOn || !result.Subscribed)
            {
                result.ErrorType = "SteamAccountPreconditionFailed";
                return result;
            }

            journal = CreateJournal(options, result, package);
            result.JournalCreated = true;
            result.Outcome = "unknown";
            ulong call = Export<CreateItemDelegate>(createItemAddress)(ugc, options.ExpectedAppId, CommunityFileType);
            result.CreateItemCallIssued = call != 0;
            journal.CallHandle = call.ToString(System.Globalization.CultureInfo.InvariantCulture);
            journal.Phase = "call_issued";
            journal.Outcome = "unknown";
            PersistJournal(journal, options.JournalPath);
            if (!result.CreateItemCallIssued)
            {
                result.ErrorType = "CreateItemCallUnavailable";
                journal.Phase = "unknown_dispatch";
                PersistJournal(journal, options.JournalPath);
                return result;
            }

            callbackBuffer = Marshal.AllocHGlobal(CreateItemResultSize);
            Stopwatch deadline = Stopwatch.StartNew();
            while (deadline.Elapsed < options.Timeout)
            {
                Export<RunCallbacksDelegate>(runCallbacksAddress)();
                byte apiCallFailed;
                byte completed = Export<IsCompletedDelegate>(completedAddress)(utils, call, out apiCallFailed);
                if (completed == 0)
                {
                    Thread.Sleep(100);
                    continue;
                }

                result.ApiCallCompleted = true;
                result.ApiCallFailed = apiCallFailed != 0;
                if (result.ApiCallFailed)
                {
                    result.ErrorType = "ApiCallFailed";
                    journal.Phase = "unknown_api_call_failed";
                    PersistJournal(journal, options.JournalPath);
                    return result;
                }

                byte resultFailed;
                result.GetApiCallResultSucceeded = Export<GetResultDelegate>(resultAddress)(
                    utils,
                    call,
                    callbackBuffer,
                    CreateItemResultSize,
                    CreateItemCallback,
                    out resultFailed) != 0;
                result.ResultRetrievalFailed = resultFailed != 0;
                if (!result.GetApiCallResultSucceeded || result.ResultRetrievalFailed)
                {
                    result.ErrorType = result.ResultRetrievalFailed
                        ? "ApiCallResultFailed"
                        : "ApiCallResultUnavailable";
                    journal.Phase = "unknown_result_retrieval";
                    PersistJournal(journal, options.JournalPath);
                    return result;
                }

                CreateItemResult callback = Marshal.PtrToStructure<CreateItemResult>(callbackBuffer);
                result.ResultCode = callback.ResultCode;
                result.PublishedFileId = callback.PublishedFileId;
                result.UserNeedsLegalAgreement = callback.UserNeedsLegalAgreement != 0;
                result.Outcome = result.ResultCode == EResultOk && result.PublishedFileId != 0
                    ? "succeeded"
                    : "failed";
                result.ErrorType = result.Outcome == "succeeded" ? null : "CreateItemRejected";
                journal.Phase = result.Outcome == "succeeded" ? "succeeded" : "callback_rejected";
                journal.Outcome = result.Outcome;
                journal.ResultCode = result.ResultCode;
                journal.PublishedFileId = result.PublishedFileId;
                journal.UserNeedsLegalAgreement = result.UserNeedsLegalAgreement;
                PersistJournal(journal, options.JournalPath);
                return result;
            }

            result.ErrorType = "CreateItemTimedOut";
            journal.Phase = "unknown_timeout";
            PersistJournal(journal, options.JournalPath);
            return result;
        }
        catch (Exception exception)
        {
            result.ErrorType = exception.GetType().Name;
            result.Outcome = journal is null ? "failed" : "unknown";
            if (journal is not null)
            {
                journal.Phase = "unknown_exception";
                journal.Outcome = "unknown";
                journal.ErrorType = result.ErrorType;
                TryPersistJournal(journal, options.JournalPath);
            }
            return result;
        }
        finally
        {
            if (initialized)
            {
                try
                {
                    Export<ShutdownDelegate>(RequiredExport(library, "SteamAPI_Shutdown"))();
                }
                catch
                {
                    // Native shutdown is best effort after the journal is durable.
                }
            }
            if (callbackBuffer != 0)
            {
                Marshal.FreeHGlobal(callbackBuffer);
            }
            if (errorBuffer != 0)
            {
                Marshal.FreeHGlobal(errorBuffer);
            }
            if (library != 0)
            {
                NativeLibrary.Free(library);
            }
        }
    }
}
