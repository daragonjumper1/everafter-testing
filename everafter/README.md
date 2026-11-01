# Everafter

A wedding planning MVP with a Rust (Axum) HTTP server, SQLite persistence, and a responsive browser interface. No Node build is needed.

For the native Windows app, launch `outputs/Everafter.exe`. Its shared Windows/Android/iOS Tauri source and platform build instructions live in [native/README.md](native/README.md). Windows has been compiled and its window inspected. Android and iOS platform builds remain unverified until their required toolchains are available.

## Run

On Windows, run the included `everafter.exe` (or `Start-Everafter.ps1`), then open http://127.0.0.1:3000. Keep the app running while using it.

To build from source, install the stable Rust toolchain from https://rustup.rs, then open a terminal in this directory:

```powershell
cargo run --release
```

Open http://127.0.0.1:3000. Start with **Wedding details**, then add guests, tasks, vendors, and vehicles. Data lives in `everafter.db` in the working directory. Back up the database while the server is stopped.

## Included

- Dashboard with actual planning, RSVP, and budget totals; no synthetic guest data.
- Manual guests and CSV import (`name,phone,email,tier`), search, RSVP filters, CSV export.
- Managed relationship categories, private support tiers, category/tier/RSVP filters, guest counts, wedding sides, and bulk group assignment.
- Private guest tokens for RSVP, mailing address, meal preference, hotel and pickup requests.
- Invitation and gift-basket delivery tracking; private planning tiers.
- Hotel and room assignments visible to the relevant guest.
- Vehicle records, arrival requests, coordinator assignments, and capacity checks in the planner UI.
- Vendor quotes, payments, decision notes, confirmation status, and due dates.
- Owned checklist tasks, completion, dates, notes, and overdue indicators.
- Wedding religion/style selection with automatically added, editable starter checklists.
- Budget totals and a prompt-driven AI invitation studio with saved images, PNG downloads, and print/PDF.

## Prompt-based invitation generation

Open **Invitation**, edit your wedding names/date/venue, and describe the artwork, palette, mood, and language. **Connect AI** accepts an OpenAI API key for this server session. The key stays in server memory; it is not written to SQLite, localStorage, logs, or generated files. Alternatively set `OPENAI_API_KEY` before starting the server. The optional `OPENAI_IMAGE_MODEL` environment variable changes the default `gpt-image-1.5` model.

Each click on **Generate invitation** sends the brief and wedding details to OpenAI's image-generation API and incurs usage on that API account. A ChatGPT subscription is separate from API billing. Account/model access and organization verification may be required. The server performs generation in a background job; the studio polls for completion and saves the image and original brief in SQLite. A restart marks unfinished jobs as failed. Existing completed designs remain available after restart. There is one concurrent generation per local app.

Download the generated PNG or print it to PDF. Check the rendered text before sending: AI can misspell names or dates. **Use this prompt** restores a previous brief to create another design; it is a fresh generation, not an image edit. Guests do not get API keys or access to this studio through their guest endpoint.

The key connection step stores a key without making a billable request; its validity and model access are checked by the provider when generating. No live provider generation was performed during development because no API key was available. Provider request formatting, image response decoding, and safe authentication-error handling are tested against a local mock HTTP server.

API implementation reference: [OpenAI Create image](https://developers.openai.com/api/reference/resources/images/methods/generate).

## Wedding-style starter checklists

In **Wedding details** (or **Checklist → Wedding style**), choose civil/secular, Hindu, Muslim/Nikah, Christian, Sikh/Anand Karaj, Jewish, Buddhist, Jain, interfaith/multicultural, or a custom ceremony. Saving adds shared planning basics and relevant ceremony discussion tasks. Additional customs can be recorded alongside the choice. Suggestions are optional planning prompts, not religious rules or legal advice; confirm actual requirements with your chosen officiant and local registrar.

Starter tasks have suggested dates based on the wedding date at the time they are added. Edit dates, owners, titles, and notes normally. Saving the same style again does not regenerate deleted tasks or duplicate tasks. Changing the style adds its suggestions while retaining previous tasks, manual edits, and completion state. Common tasks are shared across all styles. Changing only the wedding date does not move existing task dates automatically.

## Guest categories and tiers

Open **Guests → Manage groups** to create, rename, or remove categories and tiers. Default categories are Family, Friends, Colleagues, and Other. Default tiers are Inner circle (stay and travel offered), Extended circle (stay offered), and Wider circle (self-arranged). All names, priorities, descriptions, and support offers can be changed. Support offers do not automatically book anything or change a guest's stated needs.

Choose a category, side, and tier when adding/editing a guest. Filter by category, tier, and RSVP together; click a tier card to filter its members. Select guest rows and use **Organize selected** to set or clear categories and tiers in bulk. Selection can include guests hidden by current filters, and the bulk dialog makes that explicit. Rename operations preserve membership; deleting a group with assigned guests is blocked until reassignment. Categories, tiers, and sides are never returned by the private guest endpoint.

CSV import/export supports `category`, `tier`, and `side` alongside existing fields. Imported group labels are retained as managed groups. Existing free-text tier labels are preserved when the upgraded app first starts. No real guests were added during verification; API tests used an isolated local database.

## Contact import and category suggestions

Choose **Guests → Import contacts**. Supporting browsers can present the system contact picker; most desktop browsers and embedded WebViews need a CSV or UTF-8 vCard export instead. Only explicitly chosen contact fields are requested. The app never reads an entire address book silently or requests ongoing access.

The file review happens in browser memory. Explicit saved-name/group words such as “Mom”, “Cousin”, “Friend”, or “Work” produce local, editable suggestions. Plain names and conflicting labels stay uncategorized. This is a keyword heuristic, not an AI model or a verified relationship inference. There is no inference from surname, religion, ethnicity, or email domains.

Select invitees before saving. The couple can choose bride/groom/shared side at import or complete side, tier, and other details later. Unconfirmed suggestions appear under **Categories to review**; saving a guest edit or explicitly assigning a category in bulk confirms them. Unselected contacts are not sent to the server. Matching normalized phone numbers or email addresses are skipped by the server transaction; name-only contacts are not automatically merged. Different country-code representations may still need manual review. Files are limited to 5 MB/1,000 contacts per import. Older encoded vCards are rejected with an instruction to export UTF-8 instead.

See [Chrome's Contact Picker documentation](https://developer.chrome.com/docs/capabilities/web-apis/contact-picker) for platform limitations.

## Scope and deployment

For a small trusted beta, set `EVERAFTER_TEST_PASSWORD` before starting the server to require HTTP Basic authentication on every page and API route. `EVERAFTER_TEST_USER` defaults to `tester`. Passwords are read from server environment variables and are not stored in the wedding database. `EVERAFTER_DB_PATH` optionally chooses a separate SQLite database. This shared beta login gives all testers the same planner access; it does not implement separate accounts or per-couple permissions. Guest links also require the beta login when enabled.

A temporary Cloudflare Quick Tunnel can forward to a password-protected loopback instance. The link stops working when the tunnel stops and changes on restart. Keep the laptop online while testing. Never tunnel the default unprotected local server. See [Cloudflare Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/).

This is a **single-household local MVP**. The planner API has no login and is intended to run on loopback only. Guest links are functional on the same computer; remote guests cannot reach localhost. Do not expose this server to the internet as-is. A production release needs planner authentication, per-wedding authorization, HTTPS, CSRF protection, field validation, transactional server-side seat enforcement, backups, and guest-link revocation. Guest links are bearer secrets; anyone holding one can view and update that guest's allowed fields.

`EVERAFTER_BIND` can change the bind address for development. Default is `127.0.0.1:3000`. Guest responses omit other guests, tiers, planner notes, vendor records, and budgets.

Phone-native contact permissions, shared collaborator roles, automatic reminder delivery, live vendor research, and booking/payments are not integrated. Use CSV exported from your contacts; reminders are in-app dates. Hotel bookings and vehicle arrangements are recorded after being made externally. AI invitation generation requires a user-supplied API key with image-model access. The interface uses optional Google Fonts with system fallbacks.

## Verification

Compiled successfully with Rust 1.99.1 on Windows. Runtime checks passed for the homepage, records API, guest creation, private guest field filtering, RSVP update, SQLite readback, and test-record deletion. JavaScript syntax and all eight empty-state screen renders were checked. The invitation studio and connection dialog were inspected in the browser; its empty, generating, failed, and ready render states passed script checks. Three Rust tests passed for image-provider requests/responses, safe provider-error handling, and interrupted-job recovery. The live server correctly returns HTTP 428 when generation is requested without a key. Live paid image generation remains unverified without a user-supplied API key. Manually verify room and pickup assignment and try assigning more passengers than a vehicle's seats to confirm the planner blocks it.
