# Feedback revision — 2 November 2026

## Implemented in the shared app and GitHub testing demo

- Inclusive wedding-profile onboarding: partner names, optional couple type/self-description, and editable partner labels. Guest-side choices use the couple's names. This remains local onboarding, **not** server account signup or authentication.
- A Wedding section for multiple ceremonies/celebrations with dates, start/end times, venue/address, dress code, invitation notes, and a main event. Main-event details feed the invitation and planner; selected celebrations feed AI invitation requests in the Rust app.
- Budget-only total-budget editing, percentage earmarks, custom activities, reserve percentage, commitments, payments, and overspend indicators. Vendor quotes, bulk hotel bookings, and manual expenses roll up once each.
- Task prerequisite selection, ready/blocked/done/all views, blocked completion, cycle rejection, and dependency-aware starter templates. AI dependency recommendations remain a later feature.
- A single Add guests entry point with manual and contact/file-review choices. Select-all applies to reviewed/exported contacts; there is no silent full-address-book access. Bulk organization is renamed Edit selected.
- An explicit unconfirmed-category filter, clearer selection controls, separate tier-filter and tier-edit actions, and custom benefits on tiers and categories.
- Bulk accommodation bookings with stay dates, room inventory, labels, capacity, room type, price, payments, and eligible tiers. Allocation percentage, manual room assignments, and tier/preference/party-size/stay-date auto-allocation preserve existing assignments. Related hotel costs appear in Budget automatically.
- Pickup requests with pickup/drop-off addresses, Google Maps search/navigation links, flight/train references, manual travel status, manual driver ETA, grouped itineraries, transactional vehicle seat checks, and a compact Driver board. Provider buttons open Uber/Ola/Rapido websites; they are not booking integrations.

## Rust-only, requires an API key

An optional `/api/contacts/suggest` endpoint uses OpenAI structured output for provisional relationship categories. The user chooses contacts and explicitly agrees to send names and saved relationship labels; phone numbers, emails, and addresses are omitted. Plain names are insufficient evidence and protected characteristics are excluded from inference. Every suggestion requires review. This has not been exercised against a paid live API account because no key is configured.

The public Pages demo does not accept API keys and continues to use explicit local label rules. It explains this boundary in the AI-suggestion action.

## Not connected or not claimed complete

- Real signup/login, shared wedding accounts, cross-device synchronization, a securely shared driver queue, or a separate distributed driver app.
- Embedded Google Places selection returning a chosen location to the app. The current free integration opens Google Maps to find/verify addresses and navigate.
- Automatic live flight/train status, live driver GPS, or traffic-based pickup estimates. Travel updates and ETA are explicitly manual.
- Uber/Ola/Rapido booking, fare retrieval, dispatch, native in-app interfaces, or status synchronization.
- Automatic AI task-dependency suggestions.

These need additional provider access, account design, and/or a hosted backend. No paid service was activated.

## References

- [Google Maps URLs: keyless search and navigation](https://developers.google.com/maps/documentation/urls/get-started)
- [OpenAI structured model outputs](https://developers.openai.com/api/docs/guides/structured-outputs?api-mode=responses)
