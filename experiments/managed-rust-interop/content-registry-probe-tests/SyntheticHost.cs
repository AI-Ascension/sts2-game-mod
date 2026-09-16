// SPDX-License-Identifier: MIT

// Synthetic host surface for the isolated ModelDb registry probe tests.
using System;
using System.Collections.Generic;

namespace MegaCrit.Sts2.Core.Models
{
    internal readonly record struct ModelId(string Category, string Entry);

    internal abstract class AbstractModel
    {
        protected AbstractModel()
        {
            ConstructorCount++;
        }

        internal static int ConstructorCount { get; private set; }
        internal int SemanticTextReads { get; private set; }
        internal int DisplayTextReads { get; private set; }
        internal int DebugFormattingCalls { get; private set; }

        internal string SemanticText
        {
            get
            {
                SemanticTextReads++;
                return "synthetic-semantic-text";
            }
        }

        internal string DisplayText
        {
            get
            {
                DisplayTextReads++;
                return "synthetic-display-text";
            }
        }

        public override string ToString()
        {
            DebugFormattingCalls++;
            return "synthetic-debug-format";
        }

        internal static void ResetConstructorCount() => ConstructorCount = 0;
    }

    internal sealed class SyntheticCard : AbstractModel { }

    internal sealed class SyntheticRelic : AbstractModel { }

    internal sealed class SyntheticCategory { }

    internal static class ModelDb
    {
        private static readonly Dictionary<ModelId, AbstractModel> _contentById;
        internal static int InitializationCalls { get; private set; }

        static ModelDb()
        {
            AiAscension.Sts2ModelDbRegistryProbe.Tests.SyntheticModelDbInitializerMarker.CallCount++;
            _contentById = new();
        }

        internal static int Count => _contentById.Count;

        internal static void AddForTest(string category, string entry, AbstractModel model) =>
            _contentById.Add(new ModelId(category, entry), model);

        internal static void ClearForTest() => _contentById.Clear();

        internal static Type GetCategoryType(Type modelType) => typeof(SyntheticCategory);

        internal static void Init()
        {
            InitializationCalls++;
            AddForTest("card", "synthetic", new SyntheticCard());
        }
    }
}

namespace MegaCrit.Sts2.Core.Modding
{
    internal enum ModManagerState
    {
        None,
        Initialized,
        Skipped
    }
}

namespace AiAscension.Sts2ModelDbRegistryProbe.Tests
{
    internal static class SyntheticStaticConstructorMarker
    {
        internal static int CallCount;
    }

    internal static class SyntheticModelDbInitializerMarker
    {
        internal static int CallCount;
    }

    internal static class UninitializedSyntheticRegistry
    {
        private static readonly Dictionary<MegaCrit.Sts2.Core.Models.ModelId,
            MegaCrit.Sts2.Core.Models.AbstractModel> _contentById;

        static UninitializedSyntheticRegistry()
        {
            SyntheticStaticConstructorMarker.CallCount++;
            _contentById = new();
        }
    }

    internal static class WrongOwner
    {
        private static readonly Dictionary<MegaCrit.Sts2.Core.Models.ModelId,
            MegaCrit.Sts2.Core.Models.AbstractModel> _contentById = new();

        internal static object? Value => _contentById;
    }

    internal static class WrongType
    {
        private static readonly Dictionary<string, string> _contentById = new();

        internal static object? Value => _contentById;
    }

    internal static class PublicRegistry
    {
        public static readonly Dictionary<MegaCrit.Sts2.Core.Models.ModelId,
            MegaCrit.Sts2.Core.Models.AbstractModel> _contentById = new();
    }

    internal sealed class InstanceRegistry
    {
        private readonly Dictionary<MegaCrit.Sts2.Core.Models.ModelId,
            MegaCrit.Sts2.Core.Models.AbstractModel> _contentById = new();
    }
}
