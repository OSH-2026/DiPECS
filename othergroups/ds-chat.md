What kind of usage stats does Android offer in its system? (We would like to build a prototype android+ system for learning and predicting user behavior and preload certain things (etc.) when possible, and I would like to know what they already provide)
  
---

Android provides a rich set of usage statistics and prediction services—both public and internal—that you can leverage directly when building a learning/predictive “Android+” system. Here's a breakdown of what’s available.

---

## 1. Public app‑usage statistics: `UsageStatsManager`

This is the primary API for any app (with user consent) to query how and when apps are used. It gives you both aggregated summaries and a detailed event stream.

### Available data

| Data point | Method / class | Description |
|------------|----------------|-------------|
| Total time in foreground | `UsageStats.getTotalTimeInForeground()` | Cumulative time in a given interval. |
| Last time used | `UsageStats.getLastTimeUsed()` | Timestamp of the last foreground event. |
| First / last timestamp | `UsageStats.getFirstTimeStamp()`, `getLastTimeStamp()` | The monitored time range covered by the stat. |
| Package name | `UsageStats.getPackageName()` | Identifies the app. |
| Aggregation intervals | `queryUsageStats(int intervalType, long begin, long end)` | `INTERVAL_DAILY`, `WEEKLY`, `MONTHLY`, `YEARLY` – retrieve pre‑aggregated buckets. |
| Event log | `queryEvents(long begin, long end)` | Returns a sequence of `UsageEvents.Event` objects. |

### Usage event types (from `UsageEvents.Event`)

These let you reconstruct exact user‑interaction timelines:

- `MOVE_TO_FOREGROUND` / `MOVE_TO_BACKGROUND` – app transition
- `ACTIVITY_RESUMED` / `ACTIVITY_PAUSED` – per‑Activity lifecycle
- `SCREEN_INTERACTIVE` / `SCREEN_NON_INTERACTIVE` – device wake/lock
- `SHORTCUT_INVOCATION` – e.g. launcher shortcuts
- `NOTIFICATION_SEEN`, `NOTIFICATION_INTERRUPTION` – notification interactions
- `CONFIGURATION_CHANGE` – device rotation, language switch etc. while the app is visible
- `KEYGUARD_SHOWN` / `KEYGUARD_HIDDEN`
- `STANDBY_BUCKET_CHANGED` – the system’s own priority bucket for the app changed

> 💡 **Permission:** `android.permission.PACKAGE_USAGE_STATS` (user must grant in Settings → Special Access)

---

## 2. Network usage: `NetworkStatsManager`

For data‑driven preloading, you might also care about how much data each app transfers under different network conditions.

- `querySummaryForDevice(...)` – total bytes **tx/rx** per `NetworkTemplate` (e.g. all mobile)
- `queryDetailsForUid(...)` – per‑app (UID) traffic, hourly/daily buckets
- `querySummaryForUser(...)` – per‑user summary

**Permission:** `PACKAGE_USAGE_STATS` or `READ_NETWORK_USAGE_HISTORY` (the latter is signature‑level for non‑system apps)

---

## 3. Battery & system‑level stats (internal, privileged access)

If you’re building a **system image** (or a privileged app on a rooted/custom ROM), you can access far more detailed telemetry through hidden/internal services.

- **`BatteryStats` / `batterystats`**  
  Tracks per‑UID power usage, wakelocks, CPU/network/GPS time, screen state, **foreground app transitions**, etc.  
  Accessible via `dumpsys batterystats` or the internal `com.android.internal.os.BatteryStatsHelper`.  
  This is what Android uses to generate per‑app battery blame.

- **`ActivityManager` internals**  
  `getProcessMemoryInfo(int[] pids)` – per‑process detailed RAM;  
  `getHistoricalProcessExitReasons(...)` – why apps were killed.

- **`AppStandbyController`**  
  The OS categorises all apps into **buckets** (Active, Working_Set, Frequent, Rare, Restricted) based on usage recency and frequency. You can query the current bucket via `UsageStatsManager.getAppStandbyBucket()` (public API from Android 12) or read the raw internal state.

---

## 4. System –level prediction & preloading framework

Android **already** builds predictive models internally to improve performance and battery life. On a custom “Android+” system you can directly hook into or replace these components.

### 4.1 `PreloadManager` (Android 13+, system API)

This is a purpose‑built framework for **app/content preloading** based on system predictions.

- System service: `PreloadManager`
- A system‑privileged app declares a `PreloadService` and implements `onPreload(PreloadRequest request)`.
- The system calls `onPreload()` *before* it thinks you will need the content, giving you a window to fetch/prepare data.
- You update the prediction status via `PreloadManager#reportLoaded()` or `#reportError()`.

**Permission:** `android.permission.PRELOAD` (signature level)  
**Availability:** Core OS; no direct public third‑party usage, but perfect for your custom ROM prototype.

### 4.2 App launch prediction – `AppPredictionManager` (internal)

Android since 9 (Pie) has an **`AppPredictionService`** baked into the system that learns the user’s app launch sequences and context (time, location, previous app, etc.) to predict the next app(s).

- You can query predicted apps with `AppPredictionManager` (internal) or via the **`app_prediction` system service**.
- Predictions include a list of `Target` objects (usually packages) with confidence scores.
- The system uses this for preloading, App Slices ranking, and the Pixel Launcher’s “suggested” row.

In your prototype, you could:
- Directly read the predictions using the internal service.
- Feed your own signals (e.g., sensor/custom events) into the prediction pipeline by implementing a custom `AppPredictor`.

### 4.3 `JobScheduler` prefetch job (public)

Android’s `JobScheduler` has a `setPrefetch(true)` flag that tells the OS this job is for preloading content. The system then optimises scheduling (batching, network usage) and guarantees it won’t run while the device is in deep idle. This is the easiest public hook to let the OS manage your preload tasks.

```java
JobInfo job = new JobInfo.Builder(jobId, componentName)
    .setPrefetch(true)
    .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY)
    .build();
```

---

## 5. New in Android 15+: `AppStartInfo`

Android 15 added `UsageStatsManager.getAppStartInfo(int, long, long)` and the `AppStartInfo` object.  
For each app start, it records:

- **`StartReason`** – e.g. Launcher, Notification click, Broadcast, Widget, etc.
- **`Intent`** info – the action and URI, if available
- **Start timestamp**

This helps you train a predictor on *why* and *how* apps are launched, not just when.

---

## 6. Additional signals you can harvest

- **`StorageStatsManager`** – per‑app disk usage over time (needs `PACKAGE_USAGE_STATS` or system)
- **`DisplayManager`** – screen on/off, brightness changes
- **`SensorManager`** – device motion, light, proximity (context for prediction)
- **`LocationManager`** – geofence / significant motion (context)

All these can be aligned with usage event timestamps to build a rich multi‑modal dataset.

---

## Summary: what you can use out‑of‑the‑box for your prototype

| Need | Android capabilities |
|------|----------------------|
| **Learn user’s app‑usage patterns** | `UsageStatsManager` aggregates + `UsageEvents` event log |
| **Per‑app network/battery signals** | `NetworkStatsManager`, `BatteryStats` (internal) |
| **Understand why apps start** | `AppStartInfo` (Android 15+) |
| **Know the system’s own priority bucket** | `getAppStandbyBucket()` |
| **Get the system’s predicted next apps** | `AppPredictionManager` (internal) |
| **Preload content on a schedule** | `JobScheduler` with `setPrefetch(true)` |
| **Preload on system‑generated prediction** | `PreloadManager` + `PreloadService` (Android 13+, system) |
| **Extend with custom context** | All standard Android sensors + location |

For a prototype “Android+” system, a powerful approach is to combine:
- **Data collection:** `UsageEvents` with `AppStartInfo` + sensor readings for context.
- **Prediction model:** either train your own on the collected data, or feed off the internal `AppPredictionManager`.
- **Preload execution:** use `PreloadManager` (`onPreload` callback) or `JobScheduler#setPrefetch(true)` to let the OS trigger preloading at the right moment.

All these APIs are already present in AOSP; you just need to wire them together in your system image or privileged app.

---

Are there any stats that are not provided yet?

--- 

Absolutely. While Android’s built‑in usage telemetry is extensive, there are several categories of data that are not currently exposed – either because they’re considered too privacy‑sensitive, too detailed, or because they haven’t been productised as system APIs. For a prototype “Android+” system that learns user behavior and preloads content, knowing what’s missing is just as important as knowing what’s available.

Here are the major **gaps** you’ll encounter, grouped by what you’d likely need your custom ROM to fill.

---

## 1. Content‑level statistics (in‑app behavior)

| Missing stat | Why you’d want it |
|--------------|-------------------|
| Which **specific content items** the user consumed (article URL, video ID, song title, map destination, etc.) | Preload the exact media or deep‑link that will be needed next. |
| **In‑app navigation** paths (Activities, Fragments, internal screens) | Predict not just “open app X”, but “open app X → go to section Y”. |
| Scroll depth, tab switching, search queries inside an app | Gauge engagement and anticipate reloads. |
| Timestamps and durations of **media playback** per item | Prefetch the next episode or resume point. |

> 🔒 **Why missing:** These live inside the app’s own sandbox. The system only sees foreground/background transitions and UI elements exposed via Accessibility Service or Slices – there’s no global usage‑event stream for in‑app content.  
> 🛠 **What you’d do in Android+:** Implement a privileged **content‑usage tracker** that hooks into `ActivityManager`, `AccessibilityService`, or even a custom IPC shim to collect consented signals from instrumented apps.

---

## 2. Resource demand & performance profiles

| Missing stat | Purpose |
|--------------|---------|
| **Per‑app memory/CPU/IO footprint** at launch and during typical usage sessions | Decide whether preloading will cause jank or thermal throttling. |
| **Typical GPU load, codec usage** per app/session | Pre‑allocate hardware decoders or GPU buffers. |
| **App startup time breakdown** (cold/warm, class preloading, layout inflation) | Predict how long preloading will take and whether it’s worth it. |
| **Historical battery discharge rate** while using a specific app (not just total blame) | Avoid preloading when the battery is already draining fast. |

> 🔒 **Why missing:** BatteryStats and ActivityManager give aggregated CPU time/wakelocks, but not a **per‑session footprint model** that can be queried easily. The data exists internally but is not structured as a prediction‑ready stat.  
> 🛠 **In Android+:** You can extend `BatteryStats` / `perfstatsd` to record launch footprints and expose them via a new system service.

---

## 3. User attention & fine‑grained interaction

| Missing stat | Value |
|--------------|-------|
| **Touch event frequency** per app/session (taps, swipes, long‑presses) | Distinguish active use vs. idle app left open. |
| **Keyboard/IME activity** (typing speed, text input sessions) | Preload reply‑related content in messaging apps. |
| **Glance, head/face orientation** (camera‑based) | Know if the user is actually looking at the screen. |
| **Notification action rates** – which notifications are dismissed immediately vs. opened | Prioritise which app content to preload based on notify‑to‑open conversion. |

> 🔒 **Why missing:** Touch and IME events are considered highly sensitive and are not aggregated globally even for system services. Notification interaction is partially available via `UsageEvents`, but only at the “seen/interrupt” level, not at a per‑channel or per‑content level.  
> 🛠 **Android+** could add a privacy‑preserving **attention engine** that processes these signals on‑device and outputs only high‑level “engagement level” or “dismissal likelihood” metrics, never raw inputs.

---

## 4. Cross‑app task flows & intent chains

| Missing stat | Why you’d want it |
|--------------|-------------------|
| **Which app the user switches to next** after a specific app (not just launch from home). Example: email → PDF viewer → browser. | Preload the PDF viewer while the email is still in the foreground. |
| **Inter‑app intents** (Share sheet usage, links opened externally) | Anticipate that sharing a photo likely opens a messaging app. |
| **Multi‑window / split‑screen pairings** | Preload both apps when one is opened. |

> 🔒 **Why missing:** Android tracks foreground transitions but doesn’t explicitly tie them together as “task chains”. The `AppPredictionManager` learns *app launch probability* from context, but it doesn’t expose the **graph of inter‑app transitions** as raw statistics.  
> 🛠 **Your Android+** can mine `UsageEvents` to build a personalised Markov chain and expose it as a “task flow prediction” service.

---

## 5. Environmental & situational context (pre‑fused)

| Missing stat | Purpose |
|--------------|---------|
| **Activity state** (walking, driving, cycling, stationary) tied to each usage event | Preload differently when the user is commuting vs. at home. |
| **Semantic place** (home, work, gym, café) instead of raw GPS | Trigger preloads for “gym” apps when arriving. |
| **Audio environment** (noisy, quiet, music playing) | Avoid preloading heavy media on a quiet bus commute. |
| **Device posture** (folded/unfolded, tablet mode) | Preload tablet‑optimised content only when unfolded. |

> 🔒 **Why missing:** Android gives you raw sensors and the *public* activity recognition API, but it does **not** fuse them into a historical “context state” per app session that is queryable.  
> 🛠 **Android+** could run a persistent context‑fusion engine (using `ActivityRecognition`, `Geofence`, `AudioManager`, `DeviceStateManager`) and timestamp the results alongside usage events.

---

## 6. Predictive models & intent disclosure

| Missing stat | Why it’s needed |
|--------------|-----------------|
| **The system’s own predicted next‑app list with confidence scores** (publicly queryable) | Today `AppPredictionManager` is internal; you can’t just ask “which app is 85% likely next” from a normal app. |
| **Predicted **user intent** beyond app launch** (e.g., “call Mum”, “navigate to work”) | Google’s private models exist, but no external API. |
| **Pre‑computed preload window** – the system knows a good time to preload (screen off, on Wi‑Fi), but won’t tell you *why* or *when* beyond `PreloadManager` callbacks. | You may want to build your own scheduler that considers battery, network, and thermal together. |

> 🔒 **Why missing:** Intent prediction is proprietary; the system keeps the models and their confidences locked inside Google Play Services or the Launcher.  
> 🛠 **Android+** gives you the chance to build and expose your own prediction engine, possibly even feeding it the missing stats above.

---

## 7. Historical network & signal quality

| Missing stat | Purpose |
|--------------|---------|
| **Per‑app network RTT, throughput, and error rate** at time of use | Decide how aggressive to preload (e.g., avoid preloading 4K video on a spotty train connection). |
| **Cellular signal strength** trend before opening an app (the user often opens a maps app when signal drops) | Trigger offline map preload when signal is weakening. |
| **Wi‑Fi SSID / network quality** tied to usage patterns | Preload only when on “home Wi‑Fi” with known good bandwidth. |

> 🔒 **Why missing:** `ConnectivityManager` gives a real‑time bandwidth estimate, but there’s no **historical** per‑app network quality database.  
> 🛠 **Your prototype** can log `ConnectivityManager` data alongside `UsageEvents` to build its own.

---

## How to fill the gaps in your Android+ prototype

Because you’re building a custom system image, you have the unique ability to:

- Add new **instrumentation** (e.g., a `UsageTracker` system service that records these missing stats into a local database).
- Create **protected APIs** for your preload engine, optionally exposing sanitized versions to apps via new `Context`-style methods.
- Run **on‑device ML** that consumes this richer dataset and outputs predictions (app, content, resource needs) that your preload service can act on.

The missing stats boil down to a common theme: Android gives you the **what** and **when** of app launches, but not the **how, why, or what‑inside**. By instrumenting the OS deeper, you can build the complete behavioural picture that makes predictive preloading truly intelligent.

---

For instance, a user received a notification from some social media, say Lark, and opened Lark to view the received pdf file. Does current android systems provide us with the information we need?

---

For the specific flow you described — *receive Lark notification → open Lark → view a PDF attachment* — **today’s Android does not give you the full picture, but it does provide several useful pieces** that you can combine to make an educated guess. Here’s a breakdown of what you get and what you don’t.

---

## What the system **does** provide

### 1. Notification interaction events
`UsageEvents` records:
- `NOTIFICATION_SEEN` – user expanded the notification shade and the Lark notification was visible.
- `NOTIFICATION_INTERRUPTION` – the notification was dismissed (swiped away) or otherwise interrupted.

Each event includes the notification **channel ID** (e.g., `"messages"`, `"mentions"`) – but **not the notification text**, title, or attachment names.

> ✅ You can know *that a Lark notification arrived and was seen*, and on what channel.

### 2. App bring‑forward event
When the user taps the notification and Lark opens, you get:
- `MOVE_TO_FOREGROUND` for Lark’s package.
- The event timestamp, from which you can correlate how quickly after the notification the app was opened.

### 3. Start reason & deep‑link intent (Android 15+)
As of Android 15, `UsageStatsManager.getAppStartInfo()` returns an `AppStartInfo` object that tells you **why** the app started. For a notification tap, `getStartReason()` will be `START_REASON_NOTIFICATION`.  
Crucially, `getIntent()` may contain the **exact `Intent`** that launched the app, including its `action` and `data` URI.

If Lark uses meaningful deep links (e.g., `lark://message/123?attachment=report.pdf`), **you can extract the attachment filename and message thread** directly from that Intent. That’s far more detail than was previously available.

> ✅ On Android 15+, you can see the app was opened *because of a notification* and, depending on the app’s implementation, *which specific message/attachment* was targeted.

---

## What the system **does NOT** provide (and why it matters)

### ❌ 1. Actual notification content
The system does not store or expose the text, sender, or file name from the notification’s `extras`.  
Without it, you can’t know from a system‑level API that the notification was about “*John shared a PDF with you*” vs. just a like on a post. The channel ID gives only a broad category.

### ❌ 2. In‑app content consumption (the PDF view)
Once Lark is in the foreground, Android’s global usage telemetry stops at the process level. It **does not** tell you:
- That the user opened a specific message thread.
- That a PDF was loaded inside an internal viewer.
- The time spent reading that PDF.
- Whether the PDF was fully scrolled or only glanced at.

The system sees only that `com.lark.app` was in the foreground, and maybe (via `ActivityManager` internals) which top‑level `Activity` was visible, but no deeper. No “content item” granularity exists.

### ❌ 3. Explicit “notification → app” link earlier than Android 15
Before Android 15, you only had a foregound event and a nearby `NOTIFICATION_SEEN`. You could **correlate by proximity** (e.g., if Lark moved to foreground within 5 seconds of a Lark notification being seen), but that is noisy and can lead to false positives (e.g., the user might have already been using Lark and returned to it while a notification was shown).

---

## So, can you piece together the full story today?

| Piece of information | Available? | How to get it |
|----------------------|------------|---------------|
| Lark notification was seen | ✅ | `UsageEvents.NOTIFICATION_SEEN` + channel ID |
| Notification was tapped and Lark opened | ✅ (Android 15) | `AppStartInfo` with `START_REASON_NOTIFICATION` + launch Intent |
| Which PDF was involved | ⚠️ Only if the app puts a meaningful URI in the launch Intent (and Android 15) | Parse `getIntent().getData()` from `AppStartInfo` |
| User actually viewed the PDF inside Lark | ❌ | Not available; in‑app behaviour is opaque |
| How long they viewed it / whether they finished it | ❌ | No in‑app engagement metrics |

So the **short answer** is: **No, current Android does not provide all the information you need** for a complete behavioural trace. It tells you *that* a notification probably triggered a launch, and maybe the target item, but the actual content consumption (the PDF view) remains invisible.

---

## What your Android+ prototype can do to fill the gaps

Because you’re building a custom system image, you can add the missing pieces:

1. **Notification content listener** – register a `NotificationListenerService` with system privileges, log the full `Notification` extras (title, text, file names, actions) alongside the `NOTIFICATION_SEEN` timestamp.  
   → Now you know *what* the notification was about.

2. **In‑app behaviour instrumentation** – either:
   - Require apps to use a platform library that reports content‑view events (e.g., via a new system service).
   - Use an `AccessibilityService` (with appropriate security) to track screen content and interaction when the user grants consent.  
   → You’ll know they opened the PDF and for how long.

3. **Custom UsageTracker** – timestamp and fuse notification content with foreground events and accessibility traces, storing them in a local encrypted database for your on‑device prediction model.

With these additions, your Android+ system would know:  
*“At 10:03, a notification from Lark (channel ‘messages’, content: ‘John sent project.pdf’) was seen. At 10:04, Lark was opened with reason NOTIFICATION and deep link to message 123. At 10:05, the accessibility service detected a PDF viewer element labeled ‘project.pdf’ was visible for the next 5 minutes.”*  
That’s a complete, prediction‑ready signal.

---

## Bottom line

Today’s AOSP gives you the frame of the story (notification happened → app opened), and on Android 15 it even tells you why and possibly the target URI. But the **meat** — the actual content interaction — is still private to the app. For a truly predictive system that wants to preload the next PDF or resume points, you’ll need to add that observation layer yourself.