### Master Deployment Table

| Project | Type | Host | URL | Status | What I must do |
|:---|---|---|---|:---:|---|
| **01-rollback-netcode-server** | Rollback Netcode UDP Server (C++) | None | N/A | `UNDER-CONSTRUCTION` | Deploy to a dedicated Linux VPS if UDP gameplay networking is required (unsupported on free cloud tiers) |
| **02-hnsw-filtered-vector-index** | HNSW Filtered Vector Index (C++) | None | N/A | `UNDER-CONSTRUCTION` | None (Local C++ vector search kernel) |
| **03-bytecode-vm-gc** | Bytecode VM & Tracing Garbage Collector | None | N/A | `UNDER-CONSTRUCTION` | None (Local C/C++ VM runtime) |
| **04-columnar-query-engine** | Columnar Vectorized Query Engine | None | N/A | `UNDER-CONSTRUCTION` | None (Local C++ analytical query kernel) |
| **05-mini-quic** | Mini-QUIC Protocol Server (UDP) | None | N/A | `UNDER-CONSTRUCTION` | Deploy to a dedicated Linux VPS if raw UDP QUIC sockets are required |
| **06-lsm-kv-engine** | LSM-Tree Key-Value Engine (SSTable/WAL) | None | N/A | `UNDER-CONSTRUCTION` | None (Embedded C++ storage engine library) |
| **07-stream-detect-engine** | Real-Time Streaming Anomaly Engine | None | N/A | `UNDER-CONSTRUCTION` | None (Streaming anomaly detection kernel) |
| **08-pubsub-broker** | Distributed Pub/Sub Broker Core | None | N/A | `UNDER-CONSTRUCTION` | Deploy to a VPS if persistent broker daemon is needed |
| **09-distributed-cache** | Distributed In-Memory Cache Cluster | None | N/A | `UNDER-CONSTRUCTION` | Deploy cluster nodes to VPS if distributed caching is needed |
| **10-crdt-rich-text-editor** | CRDT Collaborative Editor Core | None | N/A | `UNDER-CONSTRUCTION` | None (Embedded conflict-free replication library) |
| **11-zanzibar-authz** | Zanzibar Consistent Authz Engine | None | N/A | `UNDER-CONSTRUCTION` | None (Zanzibar relationship-based authorization kernel) |
| **12-ll-hls-origin** | Low-Latency HLS Video Origin Packager | None | N/A | `UNDER-CONSTRUCTION` | None (Local video chunking packager) |
| **13-ledger-engine** | Double-Entry Ledger Core Kernel | None | N/A | `UNDER-CONSTRUCTION` | None (Core C++ financial engine) |
| **14-tracing-backend** | Distributed Tracing Python FastAPI | Render | [API](https://aj-tracing-backend.onrender.com/health) | `BACKEND-ONLY` | None (SQLite fallback active, health returns 200) |
| **15-llm-inference-scheduler** | LLM Continuous Batching Control Plane | Render | [API](https://aj-llm-inference-scheduler.onrender.com/healthz) | `BACKEND-ONLY` | None (Verified healthz 200, inference returns tokens) |
| **16-rate-limit-gateway** | Distributed Rate Limiter Gateway (TS) | Render | [API](https://aj-rate-limit-gateway.onrender.com/health) | `BACKEND-ONLY` | None (Token bucket fallback active, health returns 200) |
| **17-cdn-caching-proxy** | Edge Reverse Caching Proxy (Docker) | Render | [API](https://aj-cdn-caching-proxy.onrender.com/health) | `BACKEND-ONLY` | None (Docker container active on Render) |
| **18-flash-sale-inventory** | High-Concurrency Inventory Engine | Render | [API](https://aj-flash-sale-inventory.onrender.com/health) | `BACKEND-ONLY` | None (FastAPI inventory service active) |
| **19-usage-billing-engine** | Usage-Based Metering & Billing Engine | Render | [API](https://aj-usage-billing-engine.onrender.com/health) | `BACKEND-ONLY` | None (FastAPI billing calculations verified) |
| **20-notification-delivery-platform** | Multi-Channel Notification Dispatcher | Render | [API](https://aj-notification-delivery-platform.onrender.com/health) | `BACKEND-ONLY` | None (Multi-channel event delivery API verified) |
| **21-bis-compliance-engine** | BIS & AIS Statutory RAG Engine | Render + Pages | [App](https://avishjain9.github.io/21-bis-compliance-engine/) | `LIVE-FULL` | None (FastEmbed ONNX retrieval verified on 3 statutory queries; Pages UI active) |
| **22-repogenesis-healthcare** | Sachidanand Patient Healthcare Portal | Render + Pages | [App](https://avishjain9.github.io/22-repogenesis-healthcare/) | `LIVE-FULL` | None (Atlas MongoDB connected, user signup verified 200) |
| **23-nasa** | AstroCare Space Mission Console | Render + Pages | [App](https://avishjain9.github.io/23-nasa/) | `LIVE-FULL` | None (Atlas MongoDB time-series active, OpenAPI verified, frontend rewired) |
| **24-cooking-nexus** | Cooking Nexus Meal Planning API | Render | [API](https://aj-cooking-nexus.onrender.com/health) | `BACKEND-ONLY` | None (Express service healthy and running) |
| **25-banking** | AegisBank Ledger & Digital Banking | Render + Pages | [App](https://avishjain9.github.io/25-banking/) | `LIVE-FULL` | Provide valid Google OAuth credentials if password reset email delivery is needed |
| **26-chat-app** | Soen AI Workspace & Real-time Chat | Render + Pages | [App](https://avishjain9.github.io/26-chat-app/) | `LIVE-FULL` | None (Atlas MongoDB connected, user register verified 201) |
| **27-codealchemist** | TruthGuard Anti-Misinformation Hub | GitHub Pages | [App](https://avishjain9.github.io/27-codealchemist/) | `FRONTEND-ONLY` | None (Interactive NVIDIA-style dashboard active on Pages) |
| **28-backend-mastery** | Systems Architecture Revision Platform | GitHub Pages | [App](https://avishjain9.github.io/28-backend-mastery/) | `FRONTEND-ONLY` | None (30 curated architecture modules active on Pages) |
| **29-os-club** | Gesture PDF Presenter & Web Annotator | GitHub Pages | [App](https://avishjain9.github.io/29-os-club/) | `FRONTEND-ONLY` | None (Web client active on Pages; OpenCV native scripts local) |
| **30-blog-writing** | MERN Interactive Publishing Platform | Render + Pages | [App](https://avishjain9.github.io/30-blog-writing/) | `LIVE-FULL` | None (Atlas MongoDB connected, user signup verified 200) |
| **31-url-shortener** | Scalable URL Shortener & Analytics | Render + Pages | [App](https://avishjain9.github.io/31-url-shortener/) | `LIVE-FULL` | None (Atlas MongoDB connected, user registration verified 200) |
| **32-uber-clone** | UberRide Ride Booking Client & Dispatch | Render + Pages | [App](https://avishjain9.github.io/32-uber-clone/) | `LIVE-FULL` | None (Atlas MongoDB connected, user registration verified 201) |
| **33-hand-gesture-cv** | MediaPipe Hand Gesture Video Player | GitHub Pages | [App](https://avishjain9.github.io/33-hand-gesture-cv/) | `FRONTEND-ONLY` | None (In-browser MediaPipe vision pipeline verified) |
| **34-python-calculator** | Financial Budget & Expense Calculator | GitHub Pages | [App](https://avishjain9.github.io/34-python-calculator/) | `FRONTEND-ONLY` | None (Chart.js financial web dashboard verified on Pages) |
| **35-flappy-bird-game** | Flappy Bird Arcade Game | None | N/A | `UNDER-CONSTRUCTION` | Skipped as explicitly requested |
| **36-guess-color-game** | RGB Color Perception Web Game | GitHub Pages | [App](https://avishjain9.github.io/36-guess-color-game/) | `FRONTEND-ONLY` | None (Vanilla JS game verified on Pages) |
| **37-nature-booking-natours** | Wanderly (Natours) SASS Tour UI | GitHub Pages | [App](https://avishjain9.github.io/37-nature-booking-natours/) | `FRONTEND-ONLY` | None (Static SASS architecture verified, no Node server needed) |
| **38-forkify-recipe-app** | Forkify Modern Recipe Search Planner | GitHub Pages | [App](https://avishjain9.github.io/38-forkify-recipe-app/) | `FRONTEND-ONLY` | None (MVC client app verified on Pages) |
| **39-amazon-landing-page** | Amazon Prime E-Commerce Showcase | GitHub Pages | [App](https://avishjain9.github.io/39-amazon-landing-page/) | `FRONTEND-ONLY` | None (Responsive shopping showcase verified on Pages) |
| **40-event-hospital-website** | Sachidanand Hospital Information Portal | GitHub Pages | [App](https://avishjain9.github.io/40-event-hospital-website/) | `FRONTEND-ONLY` | None (Hospital showcase verified on Pages) |
| **41-stonehomes-vscode-zerodha** | Zerodha Kite & Stonehomes Finance UI | GitHub Pages | [App](https://avishjain9.github.io/41-stonehomes-vscode-zerodha-landings/) | `FRONTEND-ONLY` | None (Multi-landing showcase verified on Pages) |
| **42-devsprint-landing-page** | DevSprint Navigation Landing Redesign | GitHub Pages | [App](https://avishjain9.github.io/42-devsprint-landing-page-redesign/) | `FRONTEND-ONLY` | None (Interactive navigation redesign verified on Pages) |
| **43-frontend-restaurant** | Omnifood Restaurant React Experience | GitHub Pages | [App](https://avishjain9.github.io/43-frontend-restaurant/) | `FRONTEND-ONLY` | None (Omnifood React showcase active on Pages) |
| **44-starbugs-frontend-copy** | Starbucks Rewards & Menu Application | GitHub Pages | [App](https://avishjain9.github.io/44-starbugs-frontend-copy/) | `FRONTEND-ONLY` | None (Clean coffee rewards UI verified on Pages) |
| **45-trilo-travel-ui** | Trillo Luxury Hotel Booking UI | GitHub Pages | [App](https://avishjain9.github.io/45-trilo-travel-ui/) | `FRONTEND-ONLY` | None (Pure CSS Flexbox resort booking layout verified on Pages) |
| **46-testimonials-grid** | Interactive Testimonials Grid | GitHub Pages | [App](https://avishjain9.github.io/46-testimonials-grid/) | `FRONTEND-ONLY` | None (CSS Grid testimonial cards verified on Pages) |
| **47-login-auth-portal** | Express Session Auth & User Portal | Render | [API](https://aj-login-auth-portal.onrender.com/health) | `BACKEND-ONLY` | None (Session management & health checks verified on Render) |
| **48-zomato-reel-feed** | Zomato Reel & Food Video Feed | Render + Pages | [App](https://avishjain9.github.io/48-zomato-reel-feed/) | `LIVE-FULL` | None (Atlas MongoDB connected, ImageKit media active, user register verified 201) |
| **49-avishjain-weatherapp** | Glassmorphic Weather Dashboard | GitHub Pages | [App](https://avishjain9.github.io/49-avishjain-weatherapp/) | `FRONTEND-ONLY` | None (3D glassmorphic weather cards verified on Pages) |
| **50-weatherapp-client** | CP Sprint Challenge Suite | GitHub Pages | [App](https://avishjain9.github.io/50-weatherapp-client/) | `FRONTEND-ONLY` | None (Algorithmic sprint suite verified on Pages) |
| **51-css-grid-showcase** | CSS Grid Layout Revision Suite | GitHub Pages | [App](https://avishjain9.github.io/51-css-grid-showcase/) | `FRONTEND-ONLY` | None (CSS Grid layout modules verified on Pages) |
| **52-html-starter-boilerplate** | HTML5 Modern Starter Boilerplate | GitHub Pages | [App](https://avishjain9.github.io/52-html-starter-boilerplate/) | `FRONTEND-ONLY` | None (Semantic HTML5 boilerplate verified on Pages) |
| **53-first-line-of-code** | First Line of Code Experience | GitHub Pages | [App](https://avishjain9.github.io/53-first-line-of-code/) | `FRONTEND-ONLY` | None (Showcase landing verified on Pages) |

---

