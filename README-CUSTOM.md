# Custom AppView Test Client

Modified Bluesky web client for testing custom AppViews.

## Quick Start

```bash
# 1. Install dependencies
yarn install

# 2. Edit .env if needed (defaults to localhost:3000)

# 3. Start the dev server
yarn web
```

Opens at `http://localhost:19006`

## Configuration (.env)

```env
EXPO_PUBLIC_USE_CUSTOM_APPVIEW=true
EXPO_PUBLIC_CUSTOM_APPVIEW_URL=http://localhost:3000
EXPO_PUBLIC_CUSTOM_APPVIEW_DID=did:web:localhost:3000
EXPO_PUBLIC_CUSTOM_APP_NAME=AppView Test Client
EXPO_PUBLIC_CUSTOM_PDS_URL=https://bsky.social
EXPO_PUBLIC_CUSTOM_PDS_DID=did:web:bsky.social
```

**Important:** Restart dev server after changing `.env`

## What Was Changed

- `src/lib/constants.ts` - Added custom AppView/PDS support
- `.env` - Configuration for your AppView
- `package.json` - Changed name to avoid confusion

## How It Works

When `USE_CUSTOM_APPVIEW=true`:
- **Read queries** (timeline, profiles) → Your AppView
- **Authentication** → Your PDS (default: Bluesky's)
- **Writes** (posts, likes) → User's PDS

## Testing

1. **Start your AppView** on port 3000
2. **Start this client**: `yarn web`
3. **Login** with Bluesky account
4. **Browse** - data comes from your AppView!

## Switching Back

Set `EXPO_PUBLIC_USE_CUSTOM_APPVIEW=false` to use official Bluesky.
