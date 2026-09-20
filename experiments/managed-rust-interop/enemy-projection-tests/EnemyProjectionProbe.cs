// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.Rooms;

namespace AiAscension.Sts2GameMod.Runtime;

// This probe links the real IntentProjection.cs. Only external API shapes and the two
// unchanged helpers from the other native partial are substituted; no game is loaded.
internal static class EnemyProjectionProbe
{
    private static int Main()
    {
        (string Name, Action Run)[] cases =
        {
            ("observed empty remains empty", () => Equal(0, Capture().Length)),
            ("observed zero health is not unavailable", () => Equal((ushort)0, Capture(Enemy(1, 0, 0))[0].Hp)),
            ("defeated enemy remains observable", () => Equal((ushort)20, Capture(Enemy(1, 0, 20))[0].MaxHp)),
            ("ordered owned values and repeat reads", OrderedOwnedValues),
            ("maximum entity count accepted", MaximumEntities),
            ("overlarge enumeration is bounded", OversizedEnumeration),
            ("required collection getter throws", () => Unavailable(new CombatState { ReadEnemies = () => throw PrivateFailure() })),
            ("null collection is unavailable", () => Unavailable(new CombatState { ReadEnemies = () => null! })),
            ("null enemy is unavailable", () => Unavailable(new Creature[] { null! })),
            ("duplicate identity rejected", () => Unavailable(Enemy(1), Enemy(1))),
            ("invalid identity rejected", () => Unavailable(Enemy(1) with { Id = "not an identity" })),
            ("invalid required name rejected", () => Unavailable(Enemy(1) with { ReadName = () => "" })),
            ("throwing required name rejected", () => Unavailable(Enemy(1) with { ReadName = () => throw PrivateFailure() })),
            ("throwing required HP rejected", () => Unavailable(Enemy(1) with { ReadHp = () => throw PrivateFailure() })),
            ("throwing required maximum HP rejected", () => Unavailable(Enemy(1) with { ReadMaxHp = () => throw PrivateFailure() })),
            ("negative HP is not clamped", () => Unavailable(Enemy(1, -1, 20))),
            ("negative maximum HP is not clamped", () => Unavailable(Enemy(1, 0, -1))),
            ("oversized HP is not clamped", () => Unavailable(Enemy(1, 65536, 65536))),
            ("oversized maximum HP rejected", () => Unavailable(Enemy(1, 1, 65536))),
            ("HP greater than maximum rejected", () => Unavailable(Enemy(1, 21, 20))),
            ("maximum health bound preserved", () => Equal(ushort.MaxValue, Capture(Enemy(1, 65535, 65535))[0].Hp)),
            ("hidden intent preserves known HP", HiddenIntent),
            ("unavailable rendering preserves known HP", MissingRendering),
            ("throwing optional attack calculation remains unknown", UnavailableAttack),
            ("supported visible attack retains per-hit values", VisibleAttack),
            ("frozen visible intent remains unknown", FrozenIntent),
            ("compound intent remains explicitly unknown", CompoundIntent)
        };
        int passed = 0;
        try
        {
            foreach ((string name, Action run) in cases)
            {
                Reset();
                run();
                passed++;
                Console.WriteLine($"PASS {name}");
            }
            foreach (EnumerationFault fault in Enum.GetValues<EnumerationFault>())
            {
                Reset();
                using var source = new FaultingEnemies(fault);
                Unavailable(new CombatState { ReadEnemies = () => source });
                passed++;
                Console.WriteLine($"PASS enumeration fault {fault}");
            }
            Console.WriteLine($"Passed {passed} source-linked synthetic checks; native behavior unverified.");
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine($"FAIL after {passed} checks: {error.Message}");
            return 1;
        }
        finally { Reset(); }
    }

    internal static InvalidOperationException PrivateFailure() => new("PRIVATE_HOST_SENTINEL");

    internal static Creature Enemy(int id, int hp = 9, int maxHp = 20) => new()
    {
        Id = $"enemy:{id}", ReadName = () => "Synthetic enemy",
        ReadHp = () => hp, ReadMaxHp = () => maxHp
    };

    private static RuntimeV3GameplayEnemy[] Capture(params Creature[] enemies) =>
        LiveCombatSource.CaptureForProbe(new CombatState { ReadEnemies = () => enemies });

    private static void Unavailable(params Creature[] enemies) =>
        Unavailable(new CombatState { ReadEnemies = () => enemies });

    private static void Unavailable(CombatState state)
    {
        try { _ = LiveCombatSource.CaptureForProbe(state); }
        catch (InvalidOperationException error)
        {
            Equal("live enemy observation unavailable", error.Message);
            if (error.InnerException is not null || error.ToString().Contains("PRIVATE_HOST_SENTINEL", StringComparison.Ordinal))
                throw new InvalidOperationException("private failure detail was retained");
            return;
        }
        throw new InvalidOperationException("a failed read became a successful observation");
    }

    private static void OrderedOwnedValues()
    {
        Creature first = Enemy(2, 11, 30);
        Creature second = Enemy(1, 7, 40);
        RuntimeV3GameplayEnemy[] before = Capture(first, second);
        if (!before.SequenceEqual(Capture(first, second))) throw new InvalidOperationException("repeat changed data");
        Equal("enemy:2", before[0].EnemyId);
        Equal("enemy:1", before[1].EnemyId);
        first.ReadHp = () => 1;
        Equal((ushort)11, before[0].Hp);
        Equal((ushort)1, Capture(first)[0].Hp);
    }

    private static void MaximumEntities() => Equal(RuntimeV3GameplayContract.MaxEntities,
        Capture(Enumerable.Range(1, RuntimeV3GameplayContract.MaxEntities).Select(id => Enemy(id)).ToArray()).Length);

    private static void OversizedEnumeration()
    {
        int yielded = 0;
        IEnumerable<Creature> TooMany()
        {
            // Finite even against the pre-fix producer: the red test must not allocate forever.
            for (int index = 1; index <= RuntimeV3GameplayContract.MaxEntities * 2; index++)
            { yielded++; yield return Enemy(index); }
        }
        Unavailable(new CombatState { ReadEnemies = TooMany });
        Equal(RuntimeV3GameplayContract.MaxEntities + 1, yielded);
    }

    private static void HiddenIntent()
    {
        var fixture = RenderedEnemyFixture.Create();
        NCombatUi.IsDebugHidingIntent = true;
        Unknown(Capture(fixture.Enemy)[0]);
        Equal(0, fixture.DamageReads);
    }

    private static void MissingRendering()
    {
        NCombatUi.IsDebugHidingIntent = false;
        Unknown(Capture(Enemy(1))[0]);
    }

    private static void UnavailableAttack()
    {
        var fixture = RenderedEnemyFixture.Create();
        fixture.Attack.ReadDamage = () => throw PrivateFailure();
        Unknown(Capture(fixture.Enemy)[0]);
    }

    private static void VisibleAttack()
    {
        var fixture = RenderedEnemyFixture.Create();
        RuntimeV3GameplayEnemy result = Capture(fixture.Enemy)[0];
        Equal(RuntimeV3GameplayIntent.Attack, result.Intent);
        Equal((ushort)7, result.IntentDamage);
        Equal((byte)2, result.IntentHits);
        Equal(1, fixture.DamageReads);
    }

    private static void FrozenIntent()
    {
        var fixture = RenderedEnemyFixture.Create(frozen: true);
        Unknown(Capture(fixture.Enemy)[0]);
        Equal(0, fixture.DamageReads);
    }

    private static void CompoundIntent()
    {
        var fixture = RenderedEnemyFixture.Create(compound: true);
        Unknown(Capture(fixture.Enemy)[0]);
        Equal(0, fixture.DamageReads);
    }

    private static void Unknown(RuntimeV3GameplayEnemy result)
    {
        Equal((ushort)9, result.Hp);
        Equal(RuntimeV3GameplayIntent.Unknown, result.Intent);
        // Damage/hits on the internal Unknown record are not known zero attack damage.
    }

    private static void Reset() { NCombatRoom.Instance = null; NCombatUi.IsDebugHidingIntent = true; }

    private static void Equal<T>(T expected, T actual)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
            throw new InvalidOperationException($"expected {expected}, received {actual}");
    }
}

internal sealed partial class LiveCombatSource
{
    internal static RuntimeV3GameplayEnemy[] CaptureForProbe(CombatState state) => ProjectEnemiesSafely(state);
    private static string EnemyId(Creature enemy) => enemy.Id;
    // Retained solely so this probe also compiles against the pre-fix source for a red test.
    private static ushort U16(int value) => (ushort)Math.Clamp(value, 0, ushort.MaxValue);
}
