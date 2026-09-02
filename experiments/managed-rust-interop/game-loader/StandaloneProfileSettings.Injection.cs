// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Godot;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.Screens.Settings;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static readonly BindingFlags PrivateInstance = BindingFlags.Instance | BindingFlags.NonPublic;

    private static void OnNodeAdded(Node node)
    {
        if (node is not NSettingsTabManager tabManager || node.GetNodeOrNull(TabName) != null)
        {
            return;
        }

        node.Connect(
            "ready",
            Callable.From(() => InjectProfileTab(tabManager)),
            (uint)GodotObject.ConnectFlags.OneShot);
    }

    private static void InjectProfileTab(NSettingsTabManager tabManager)
    {
        try
        {
            FieldInfo? tabsField = typeof(NSettingsTabManager).GetField("_tabs", PrivateInstance);
            if (tabsField?.GetValue(tabManager) is not IDictionary tabs || tabs.Count == 0)
            {
                GD.PrintErr($"{LogPrefix} could not find the host settings tab dictionary");
                return;
            }

            NSettingsTab? firstTab = null;
            NSettingsPanel? firstPanel = null;
            foreach (DictionaryEntry entry in tabs)
            {
                firstTab = entry.Key as NSettingsTab;
                firstPanel = entry.Value as NSettingsPanel;
                if (firstTab != null && firstPanel != null) break;
            }

            if (firstTab == null || firstPanel == null)
            {
                GD.PrintErr($"{LogPrefix} could not find a host settings tab to clone");
                return;
            }

            var profileTab = (NSettingsTab)firstTab.Duplicate();
            profileTab.Name = TabName;
            if (profileTab.GetNodeOrNull<TextureRect>("TabImage")?.Material is ShaderMaterial shader)
            {
                profileTab.GetNode<TextureRect>("TabImage").Material = (ShaderMaterial)shader.Duplicate();
            }

            tabManager.AddChild(profileTab);
            profileTab.SetLabel(TabLabel);
            profileTab.Deselect();
            PositionNewTab(tabs, profileTab, tabManager);

            var profilePanel = (NSettingsPanel)firstPanel.Duplicate();
            profilePanel.Name = PanelName;
            profilePanel.Visible = false;
            Control? preReadyFocusSentinel = CreatePreReadyFocusSentinel(firstPanel);
            string? contentName = firstPanel.Content?.Name;
            VBoxContainer? contentContainer = null;

            foreach (Node child in profilePanel.GetChildren().ToArray())
            {
                bool keepAsContent = child is VBoxContainer candidate
                    && ((contentName != null && candidate.Name == contentName)
                        || (contentName == null && contentContainer == null));

                if (keepAsContent && child is VBoxContainer content)
                {
                    contentContainer = content;
                    foreach (Node inner in content.GetChildren().ToArray())
                    {
                        content.RemoveChild(inner);
                        inner.Free();
                    }
                }
                else
                {
                    profilePanel.RemoveChild(child);
                    child.Free();
                }
            }

            if (contentContainer != null && preReadyFocusSentinel != null)
            {
                preReadyFocusSentinel.Name = "__PreReadyFocusSentinel";
                preReadyFocusSentinel.Visible = false;
                preReadyFocusSentinel.MouseFilter = Control.MouseFilterEnum.Ignore;
                contentContainer.AddChild(preReadyFocusSentinel);
            }

            Node? panelParent = firstPanel.GetParent();
            if (panelParent == null)
            {
                throw new InvalidOperationException("host settings panel has no parent");
            }

            panelParent.AddChild(profilePanel);
            contentContainer ??= profilePanel.Content;
            tabs.Add(profileTab, profilePanel);

            profileTab.Connect(
                NClickableControl.SignalName.Released,
                Callable.From<NButton>(_ => tabManager.Call("SwitchTabTo", profileTab)));

            CacheGameFont(firstPanel);
            PopulateProfileSettings(contentContainer);
            RebuildFocusTargets(profilePanel, contentContainer, preReadyFocusSentinel);
            GD.Print($"{LogPrefix} standalone profile settings tab installed");
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} standalone profile settings failed: {exception.GetType().Name}");
        }
    }

    private static void PositionNewTab(IDictionary tabs, NSettingsTab profileTab, NSettingsTabManager tabManager)
    {
        List<NSettingsTab> existingTabs = new();
        foreach (DictionaryEntry entry in tabs)
        {
            if (entry.Key is NSettingsTab tab) existingTabs.Add(tab);
        }

        if (existingTabs.Count == 0) return;

        profileTab.Size = existingTabs[0].Size;
        float spacing = existingTabs.Count > 1
            ? existingTabs[1].Position.X - existingTabs[0].Position.X
            : existingTabs[0].Size.X;
        if (spacing <= 0) spacing = existingTabs[0].Size.X;

        NSettingsTab lastTab = existingTabs[^1];
        profileTab.Position = new Vector2(lastTab.Position.X + spacing, lastTab.Position.Y);

        float rightEdge = profileTab.Position.X + profileTab.Size.X;
        if (tabManager.Size.X <= 0 || rightEdge <= tabManager.Size.X) return;

        int totalTabs = existingTabs.Count + 1;
        float newSpacing = tabManager.Size.X / totalTabs;
        float startX = (newSpacing - existingTabs[0].Size.X) / 2f;
        for (int i = 0; i < existingTabs.Count; i++)
        {
            existingTabs[i].Position = new Vector2(startX + newSpacing * i, existingTabs[i].Position.Y);
        }

        profileTab.Position = new Vector2(startX + newSpacing * existingTabs.Count, existingTabs[0].Position.Y);
    }
}
