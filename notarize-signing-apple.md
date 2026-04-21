# macOS Code Signing and Notarization for CLAP Plugins

A complete walkthrough for signing and notarizing a CLAP audio plugin for distribution outside the Mac App Store.

## Prerequisites

- An Apple Developer account (paid, $99/year): https://developer.apple.com
- Xcode Command Line Tools: `xcode-select --install`
- Rust toolchain: https://rustup.rs

---

## Step 1: Generate a Certificate Signing Request (CSR)

Do this from the terminal — the Keychain Access GUI Certificate Assistant is unreliable.

```bash
# Generate a private key
openssl genrsa -out ~/keys/developer_id_app.key 2048

# Generate the CSR
openssl req -new \
  -key ~/keys/developer_id_app.key \
  -out ~/keys/developer_id_app.certSigningRequest \
  -subj "/emailAddress=you@example.com/CN=Your Name/C=US"
```

Keep `~/keys/` backed up securely — the private key cannot be recovered if lost.

If you need a separate Installer certificate, generate a second key pair:

```bash
openssl genrsa -out ~/keys/developer_id_installer.key 2048
openssl req -new \
  -key ~/keys/developer_id_installer.key \
  -out ~/keys/developer_id_installer.certSigningRequest \
  -subj "/emailAddress=you@example.com/CN=Your Name/C=US"
```

---

## Step 2: Request Certificates from Apple

Go to: https://developer.apple.com/account/resources/certificates/add

### Developer ID Application (required for signing distributable binaries/bundles)

1. Under the **Developer ID** section, select **Developer ID Application**
2. Click Continue
3. Select **G2 Sub-CA (Xcode 11.4.1 or later)** — this is the current standard
4. Upload `developer_id_app.certSigningRequest`
5. Download the resulting `developerID_application.cer`

### Developer ID Installer (required for signing .pkg installers)

1. Go back to Add Certificate
2. Under the **Developer ID** section, select **Developer ID Installer**
3. Click Continue, select **G2 Sub-CA**
4. Upload `developer_id_installer.certSigningRequest` (a fresh CSR — Apple does not allow reuse)
5. Download the resulting `developerID_installer.cer`

**Important:** Make sure you are selecting from the **Developer ID** section, not the
"Software" or "Production" sections, which produce App Store certificates instead.
The correct certificates will have `CN=Developer ID Application` or
`CN=Developer ID Installer` baked into them. You can verify with:

```bash
openssl x509 -in developerID_application.cer -inform DER -noout -subject
```

---

## Step 3: Import Certificates into Your Keychain

Import each private key paired with its certificate:

```bash
# Application cert
security import ~/keys/developer_id_app.key \
  -k ~/Library/Keychains/login.keychain-db \
  -T /usr/bin/codesign

security import ~/keys/developerID_application.cer \
  -k ~/Library/Keychains/login.keychain-db

# Installer cert
security import ~/keys/developer_id_installer.key \
  -k ~/Library/Keychains/login.keychain-db \
  -T /usr/bin/productbuild

security import ~/keys/developerID_installer.cer \
  -k ~/Library/Keychains/login.keychain-db
```

Verify both identities are ready:

```bash
security find-identity -v | grep "Developer ID"
# Should show both Application and Installer entries
```

---

## Step 4: Generate an App-Specific Password for Notarization

Apple requires an app-specific password (not your Apple ID password) for notarization tools.

1. Sign in at https://account.apple.com with your Apple ID
2. Go to **Sign-In and Security → App-Specific Passwords**
3. Click **Generate an app-specific password** and give it a label (e.g. `notarytool`)
4. Apple generates a password in the format `xxxx-xxxx-xxxx-xxxx` — copy it immediately
5. Note: two-factor authentication must be enabled on your Apple account

---

## Step 5: Store Notarization Credentials in Your Keychain

Do this once. It saves your credentials securely so you don't need to pass them on every build.

```bash
xcrun notarytool store-credentials "my-notary-profile" \
  --apple-id you@example.com \
  --team-id XXXXXXXXXX \
  --password xxxx-xxxx-xxxx-xxxx
```

Your Team ID is the 10-character alphanumeric string shown next to your team name on
https://developer.apple.com/account — or visible in `security find-identity -v`.

---

## Step 6: Sign the Bundle

Sign inner binaries before outer bundles (inside-out order):

```bash
SIGN_ID="Developer ID Application: Your Name (TEAMID)"

codesign --force --sign "$SIGN_ID" \
  --options runtime \
  --timestamp \
  --strict \
  MyPlugin.clap/Contents/MacOS/MyPlugin

codesign --force --sign "$SIGN_ID" \
  --options runtime \
  --timestamp \
  --strict \
  --identifier "com.example.myplugin" \
  MyPlugin.clap

# Verify
codesign --verify --deep --strict --verbose=2 MyPlugin.clap
```

---

## Step 7: Notarize the Bundle

Zip it (Apple requires a zip or dmg for submission, not a bare bundle):

```bash
ditto -c -k --keepParent MyPlugin.clap MyPlugin.zip

xcrun notarytool submit MyPlugin.zip \
  --keychain-profile "my-notary-profile" \
  --wait
```

`--wait` polls until Apple finishes (usually under 2 minutes). On success, staple the
ticket so Gatekeeper can verify offline:

```bash
xcrun stapler staple MyPlugin.clap
```

---

## Step 8: Build and Sign a .pkg Installer (optional)

Stage the bundle at its install location, then use `pkgbuild`:

```bash
INSTALLER_ID="Developer ID Installer: Your Name (TEAMID)"

mkdir -p stage/Library/Audio/Plug-Ins/CLAP
cp -R MyPlugin.clap stage/Library/Audio/Plug-Ins/CLAP/

pkgbuild \
  --root stage \
  --identifier "com.example.myplugin.pkg" \
  --version "1.0.0" \
  --install-location "/" \
  --sign "$INSTALLER_ID" \
  --timestamp \
  MyPlugin.pkg

pkgutil --check-signature MyPlugin.pkg
```

Then notarize and staple the .pkg the same way as the bundle:

```bash
ditto -c -k --keepParent MyPlugin.pkg MyPlugin_pkg.zip

xcrun notarytool submit MyPlugin_pkg.zip \
  --keychain-profile "my-notary-profile" \
  --wait

xcrun stapler staple MyPlugin.pkg
```

---

## Automation

The `build.sh` in this repo automates all of the above:

```bash
# Sign + notarize the .clap bundle
NOTARY_PROFILE=my-notary-profile ./build.sh

# Also build a signed + notarized .pkg installer
NOTARY_PROFILE=my-notary-profile ./build.sh --pkg

# Sign only, skip notarization
./build.sh --no-notarize

# Install the .clap to ~/.clap/ after building
NOTARY_PROFILE=my-notary-profile ./build.sh --install
```
