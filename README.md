# Everafter — wedding planner test build

This repository contains the Rust application, a Windows/Android/iOS Tauri shell, and an interactive **browser-only GitHub Pages demo** for usability testing.

## Try the demo

After Pages is enabled, the site is `https://daragonjumper1.github.io/everafter-testing/`. The **Feedback** button opens this repository's bug-report and improvement templates. Friends can use GitHub Issues and comment on existing reports. A GitHub account is needed to submit an Issue.

The public demo uses fictional sample guests and stores changes **only in each browser's localStorage**. Testers do not share a wedding or a database. A reset button restores the sample wedding. Use fictional contact details during public testing. No real database, API key, or test password is included in this repository.

See [the feedback changelog](FEEDBACK-CHANGELOG.md) for the latest implemented changes and service-dependent gaps.

### Available in the Pages demo

- Compact phone UI and desktop layout, bottom navigation, dashboard, and task tabs.
- Guest CRUD, CSV/vCard import and review, category suggestions from explicit saved labels, categories/tiers, bulk grouping, and RSVP status editing by the planner.
- Hotel/room records, vehicles and assignments, vendors, payment totals, and budgets.
- Inclusive local onboarding, multi-event wedding details, dependency-aware tasks, percentage budget earmarks, bulk accommodation/auto-allocation, itinerary grouping, and a driver-board prototype.
- Wedding-style starter checklists exported directly from the Rust implementation.
- Invitation prompt editor and navigation, with an honest unavailable state for live generation.

### Requires the Rust backend or additional work

- Actual AI image generation, image persistence, and private guest RSVP links are **not available** on Pages. No API keys are accepted by the public demo.
- There is no shared login, cloud synchronization, or multi-tester shared data.
- Device contact-picker support depends on the browser; CSV/vCard is the fallback. Native phone permission integrations are not implemented.
- The demo's localStorage adapter exercises UI behavior; it is not a replacement for Rust API/security/integration tests.

## Publish with GitHub Pages — free public-repository workflow

1. Create a new **public** repository and upload these files to `main` (including `.github`).
2. Open **Settings → Pages → Build and deployment → Source → GitHub Actions**.
3. Open **Actions → Publish interactive test demo → Run workflow**, or push a change to `docs/`.
4. Wait for the deployment to succeed, then open the Pages URL shown by the workflow.

The workflow fills in the repository URL used by the Feedback button. Issues must be enabled under the repository's general settings. There is no chargeable hosting service or third-party API key required for this demo.

## Run the real Rust app

From `everafter/`, run `cargo run --release`, then open `http://127.0.0.1:3000`. See `everafter/README.md` and `everafter/native/README.md` for complete app and native build instructions.

## Update the demo

The `docs/` assets are a prepared snapshot. Run `node scripts/build-demo.mjs` after shared UI changes. It copies the source assets and retains the demo-specific API adapter, relative URLs, and disabled API-key/guest-link behavior. Refresh checklist templates with:

```text
cargo run --manifest-path everafter/Cargo.toml --bin export-demo-checklists -- docs/checklist-templates.json
```

Wrap that JSON as `window.EVERAFTER_STARTERS=<JSON>;` in `docs/checklist-templates.js`.

## Testing and feedback

Use Issues for reproducible bugs and general comments. Include the screen name, steps, expected result, device/browser, and a screenshot without personal data. Known gaps such as AI generation and remote RSVP links are intentional demo boundaries, not server regressions.

GitHub documentation: [Pages overview and availability](https://docs.github.com/en/pages/getting-started/what-is-github-pages) and [creating a Pages site](https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site).
