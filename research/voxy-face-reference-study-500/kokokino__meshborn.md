# Facial rendering source scan: kokokino/meshborn

Repository: https://github.com/kokokino/meshborn

Pinned source revision: 4a16c45008b71eb2b59e778ad701dd7fad4f2a87

Discovery topic: tongue animation

Description: Build a web service that takes a text prompt or image and outputs a complete, TalkingHead-ready GLB avatar — with split lips, teeth, tongue, eyes, 52 ARKit blend shapes, 15 Oculus visemes, a Mixamo-compatible rig, and animations — all automatically. Essentially: the child of Ready Player Me and Tripo AI, but fully automated and AI-generated.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {}

## Source file: client/main.js

{
  "path": "client/main.js",
  "blob_sha": "425d7805a069416c16e21a61a126659618a66b05",
  "sha256": "9875255faab9e5c6badd7dab422e4fe73712ec9b5a7126ee48930e60f1469176",
  "lines": 84,
  "bytes": 2086,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kokokino/meshborn/blob/4a16c45008b71eb2b59e778ad701dd7fad4f2a87/client/main.js",
  "function_signature_candidates": []
}

```text

import m from 'mithril';
import { Meteor } from 'meteor/meteor';
import { Tracker } from 'meteor/tracker';
import './main.html';

// Import Pico CSS directly from node_modules
import '@picocss/pico/css/pico.min.css';

// Import pages
import { MainLayout } from '../imports/ui/layouts/MainLayout.js';
import { HomePage } from '../imports/ui/pages/HomePage.js';
import { NotLoggedIn } from '../imports/ui/pages/NotLoggedIn.js';
import { NoSubscription } from '../imports/ui/pages/NoSubscription.js';
import { SessionExpired } from '../imports/ui/pages/SessionExpired.js';
import { SsoCallback } from '../imports/ui/pages/SsoCallback.js';

// Import collections for subscriptions
import '../imports/lib/collections/chatMessages.js';

// Reactive wrapper for Mithril
// This ensures Mithril redraws when Meteor reactive data changes
const MeteorWrapper = {
  oninit() {
    this.computation = null;
  },
  oncreate() {
    this.computation = Tracker.autorun(() => {
      Meteor.user();
      Meteor.userId();
      Meteor.loggingIn();
      m.redraw();
    });
  },
  onremove() {
    if (this.computation) {
      this.computation.stop();
    }
  },
  view(vnode) {
    return vnode.children;
  }
};

// Layout wrapper component
const Layout = {
  view(vnode) {
    return m(MeteorWrapper, m(MainLayout, vnode.attrs, vnode.children));
  }
};

// Route resolver that wraps pages in layout
function layoutRoute(component, attrs = {}) {
  return {
    render() {
      return m(Layout, attrs, m(component));
    }
  };
}

// Initialize Mithril routing
function initializeApp() {
  // Create #app element if it doesn't exist
  let root = document.getElementById('app');
  
  // Set up routes
  m.route.prefix = '';

  m.route(root, '/', {
    '/': layoutRoute(HomePage),
    '/not-logged-in': layoutRoute(NotLoggedIn),
    '/no-subscription': layoutRoute(NoSubscription),
    '/session-expired': layoutRoute(SessionExpired),
    '/sso': {
      render() {
        return m(SsoCallback);
      }
    }
  });
}

// Initialize app when DOM is ready
Meteor.startup(() => {
  initializeApp();
});


```

## Source file: server/main.js

{
  "path": "server/main.js",
  "blob_sha": "c64e7295f93908babc52d8cdb79a9d862c363e8f",
  "sha256": "9ccbd1a84b4ce5367abd15d5ed4358f535baa8fdd25d0862daa43de244050177",
  "lines": 28,
  "bytes": 744,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kokokino/meshborn/blob/4a16c45008b71eb2b59e778ad701dd7fad4f2a87/server/main.js",
  "function_signature_candidates": []
}

```text

import { Meteor } from 'meteor/meteor';

// Import server modules
import './accounts.js';
import './publications.js';
import './methods.js';
import './indexes.js';
import './rateLimiting.js';
import './migrations/0_steps.js';

Meteor.startup(async () => {
  console.log('Spoke App Skeleton started');
  
  // Verify required settings are present
  const settings = Meteor.settings;
  
  if (!settings.public?.hubUrl) {
    console.warn('Warning: settings.public.hubUrl is not configured');
  }
  
  if (!settings.private?.hubApiKey) {
    console.warn('Warning: settings.private.hubApiKey is not configured');
  }
  
  if (!settings.private?.hubPublicKey) {
    console.warn('Warning: settings.private.hubPublicKey is not configured');
  }
});


```

## Source file: rspack.config.js

{
  "path": "rspack.config.js",
  "blob_sha": "33f02e3bbec98f1bc5a8816c1d15f435c50407df",
  "sha256": "1c834e1390469bf9fd490614c63b6f474e42a191f2f0c48a1c6a289b2c8455e7",
  "lines": 15,
  "bytes": 430,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kokokino/meshborn/blob/4a16c45008b71eb2b59e778ad701dd7fad4f2a87/rspack.config.js",
  "function_signature_candidates": []
}

```text

const { defineConfig } = require('@meteorjs/rspack');

/**
 * Rspack configuration for Meteor projects.
 *
 * Provides typed flags on the `Meteor` object, such as:
 * - `Meteor.isClient` / `Meteor.isServer`
 * - `Meteor.isDevelopment` / `Meteor.isProduction`
 * - …and other flags available
 *
 * Use these flags to adjust your build settings based on environment.
 */
module.exports = defineConfig(Meteor => {
  return {};
});


```

## Source file: server/indexes.js

{
  "path": "server/indexes.js",
  "blob_sha": "f1d22fff86866f631a1b9f02454deffb2e7325a6",
  "sha256": "c7f6d7a1f9e625a08baf18774335828aef1b6f1c8879e512d9ff10ff01abab2a",
  "lines": 19,
  "bytes": 563,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kokokino/meshborn/blob/4a16c45008b71eb2b59e778ad701dd7fad4f2a87/server/indexes.js",
  "function_signature_candidates": []
}

```text

import { Meteor } from 'meteor/meteor';
import { UsedNonces, SubscriptionCache } from '../imports/api/collections.js';

Meteor.startup(async () => {
  // SSO user lookup index
  await Meteor.users.createIndexAsync({ 'services.sso.hubUserId': 1 });

  // TTL index: auto-delete nonces after 10 minutes
  await UsedNonces.createIndexAsync(
    { createdAt: 1 },
    { expireAfterSeconds: 600 }
  );

  // TTL index: auto-delete cache entries after 5 minutes
  await SubscriptionCache.createIndexAsync(
    { createdAt: 1 },
    { expireAfterSeconds: 300 }
  );
});


```
