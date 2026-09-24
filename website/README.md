# hushtype website

Static site for [hushtype.vercel.app](https://hushtype.vercel.app): `index.html`, `styles.css`, `app.js`, self-hosted fonts (SIL Open Font License, licenses in `fonts/`), and the installer `install.sh`. There is no build step, no analytics, and no third-party request; `vercel.json` sets a strict Content-Security-Policy.

## Release

1. Build the app (see the repository README).
2. If dependencies changed, run `scripts/generate-third-party-notices.sh`.
3. Run `scripts/package-website-release.sh`. It zips the built app into `download/` (git-ignored), writes the zip's SHA-256 and size into `install.sh`, and copies `THIRD_PARTY_NOTICES` to `notices.txt`.
4. Deploy from this directory: `vercel deploy --prod --scope sidebandstudio`.
5. Check that `curl -fsSL https://hushtype.vercel.app/install.sh | bash` installs the new version.

Keep one app version per zip name. Zips are served with an immutable cache header, so a changed build needs a new version number.

## Installer

`install.sh` runs everything inside `main()`, so a partial download does nothing. It checks for Apple silicon, macOS 14 or later, and 1.5 GB of free space; downloads the app and the Parakeet V3 model; verifies each file's SHA-256; moves hushtype into `/Applications` (or `~/Applications`); and opens it. On an update it closes the running app and clears only hushtype's Accessibility entry, because macOS ties that permission to the exact build.

The Terminal install exists because hushtype is not notarized: macOS blocks a browser-downloaded, ad-hoc signed app. With a Developer ID certificate and notarization, a normal DMG download would work too.

## Domain

The site answers at hushtype.vercel.app. To serve it at `hushtype.sideband.studio`, add the domain to the `hushtype` Vercel project and create a `CNAME` record for `hushtype` pointing to `cname.vercel-dns.com` at the DNS provider for sideband.studio, then update the URLs in `index.html` and `install.sh`.
