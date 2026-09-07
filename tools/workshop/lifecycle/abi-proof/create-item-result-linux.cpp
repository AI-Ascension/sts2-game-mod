// SPDX-License-Identifier: MIT
// Compile this translation unit against the pinned Valve public headers.

#include <cstddef>
#include <cstdint>

#include "steamclientpublic.h"
#include "isteamugc.h"

static_assert(sizeof(EResult) == sizeof(std::int32_t));
static_assert(sizeof(PublishedFileId_t) == sizeof(std::uint64_t));
static_assert(CreateItemResult_t::k_iCallback == k_iSteamUGCCallbacks + 3);
static_assert(offsetof(CreateItemResult_t, m_eResult) == 0);
static_assert(offsetof(CreateItemResult_t, m_nPublishedFileId) == 4);
static_assert(offsetof(CreateItemResult_t, m_bUserNeedsToAcceptWorkshopLegalAgreement) == 12);
static_assert(sizeof(CreateItemResult_t) == 16);

int main() { return 0; }
