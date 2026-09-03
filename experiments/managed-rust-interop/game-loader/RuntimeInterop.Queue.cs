// SPDX-License-Identifier: MIT

using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static void ProcessRuntimeQueue()
    {
        for (int index = 0; index < 16 && RuntimeQueue.TryDequeue(out RuntimeWork? work); index++)
        {
            System.Threading.Interlocked.Decrement(ref _runtimeQueueDepth);
            if (!work.TryClaimForProcessing())
            {
                work.Status = RuntimeTimeout;
                work.Response = RuntimeError(work.Context, work.Kind, "main_thread_timeout");
                work.Completed.Set();
                continue;
            }

            try
            {
                (work.Status, work.Response) = ProcessRuntimeWork(work);
            }
            catch (System.Exception exception)
            {
                work.Status = RuntimeUnavailable;
                work.Response = RuntimeError(work.Context, work.Kind, "main_thread_exception");
                GD.PrintErr($"{LogPrefix} runtime main-thread request failed: {exception.GetType().Name}: {exception.Message}");
            }
            finally
            {
                work.Completed.Set();
            }
        }

        TryFinalizePendingRuntimeV2();
        TryFinalizePendingRuntimeV3Gameplay();
    }

    private static bool TryEnqueueRuntimeWork(RuntimeWork work)
    {
        int depth = System.Threading.Interlocked.Increment(ref _runtimeQueueDepth);
        if (depth > RuntimeQueueCapacity())
        {
            System.Threading.Interlocked.Decrement(ref _runtimeQueueDepth);
            return false;
        }

        RuntimeQueue.Enqueue(work);
        return true;
    }

    private static int RuntimeQueueCapacity()
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimeQueueCapacityVariable);
        return int.TryParse(value, out int capacity)
            && capacity >= 1
            && capacity <= RuntimeQueueMaximumCapacity
            ? capacity
            : RuntimeQueueDefaultCapacity;
    }
}
