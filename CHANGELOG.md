# Changelog

## [0.4.0](https://github.com/crimsonsunset/mcp-mux/compare/v0.3.0...v0.4.0) (2026-07-01)


### Features

* add default_params_strategy, tool_arguments alias, and gateway health improvements ([d6e4554](https://github.com/crimsonsunset/mcp-mux/commit/d6e45549be1f331a565a9d8528997407b98ce83e))
* **backend:** Phase 1 — Scaffold + enforcement ([1f36ad9](https://github.com/crimsonsunset/mcp-mux/commit/1f36ad90e4c52c51f5cc24f9b76c1f89242c270a))
* **backend:** Phase 2 — Events facade ([f72af83](https://github.com/crimsonsunset/mcp-mux/commit/f72af8323b52257fa65f49f61e562ea8a8b3eb23))
* **backend:** Phase 3 — Shell facade + autonomous decisions ([e9a0e49](https://github.com/crimsonsunset/mcp-mux/commit/e9a0e497c7ca9334fbae48c260e03d852f4b9f2c))
* **backend:** Phase 4 — Data stragglers + autonomous decisions ([90cbd9f](https://github.com/crimsonsunset/mcp-mux/commit/90cbd9fd15c6f23d7b75d60502645e39811890f6))
* **consent-model:** FeatureSet bind consent, hybrid search, and embedding cache ([444b83a](https://github.com/crimsonsunset/mcp-mux/commit/444b83ae6ffa320543e4ed171960d23bcd78bb91))
* **consent-model:** Phase 1 — Discovery surfaces inactive capability ([5529aee](https://github.com/crimsonsunset/mcp-mux/commit/5529aee2581adfeb6c0352d731ada72bc2308c72))
* **consent-model:** Phase 2 — Bind becomes canonical activation path ([dfa00f2](https://github.com/crimsonsunset/mcp-mux/commit/dfa00f26dd1934b67ce87748884aa79c2aeb7503))
* **consent-model:** Phase 3 — Remove ephemeral session-override path ([2f5f967](https://github.com/crimsonsunset/mcp-mux/commit/2f5f9673b99f8717a3fbd3d52e65a8deb399b612))
* **consent-model:** Phase 4 — Lock authoring to humans ([55cdfd2](https://github.com/crimsonsunset/mcp-mux/commit/55cdfd210f7f20f1497de72826942afefb7994fa))
* **consent-model:** Phase 5 — Approval rendering in web client ([ae3833a](https://github.com/crimsonsunset/mcp-mux/commit/ae3833af3e5bfd7fe3c2dae3b4357ec112749c1c))
* **core,storage:** schema for FeatureSet resolver v2 (forward-compat) ([d16d50d](https://github.com/crimsonsunset/mcp-mux/commit/d16d50dc33358c7a669432589b5c34a804affe92))
* **dashboard:** clickable health rows with log viewer, Esc to close, and pre-filtered error nav ([8490471](https://github.com/crimsonsunset/mcp-mux/commit/84904710f67c3440e8bc57353c9faf4b67a283ba))
* **dashboard:** expand home view with health, activity, and quick links ([7b57841](https://github.com/crimsonsunset/mcp-mux/commit/7b57841af6063169d2c70d3c381250399b02fa85))
* **desktop:** add copy-all to server log viewer ([36a4922](https://github.com/crimsonsunset/mcp-mux/commit/36a4922696786f66add7e3fbe4d37820e83b641d))
* **desktop:** add server count summary and smarter hover tooltips ([6215633](https://github.com/crimsonsunset/mcp-mux/commit/621563309a5d2e4813163411cb11f6441f120a3e))
* **desktop:** expand/collapse all servers and disable on error ([8ff352b](https://github.com/crimsonsunset/mcp-mux/commit/8ff352bbf7198787d18c35cd70ee2a0c9ea00218))
* **desktop:** My Servers search, filters popover, and shared UI primitives ([b641acd](https://github.com/crimsonsunset/mcp-mux/commit/b641acd95bdf983a6a7b0974ee702eea8d0ac9c3))
* **dev:** add web admin dev scripts and macOS run-from-source docs ([f79229b](https://github.com/crimsonsunset/mcp-mux/commit/f79229b1e4bbb69598e9616f0b4747f2baf4971d))
* FeatureSet resolver v2 — authoritative + UI + migration 003 ([ecf6bb8](https://github.com/crimsonsunset/mcp-mux/commit/ecf6bb8b7b5d03286a1b4b62105b25f7e730e918))
* **featuresets:** add select/deselect all for included features ([0e22c23](https://github.com/crimsonsunset/mcp-mux/commit/0e22c2367f191d398469370f3b2835e98c2c7703))
* **gateway,desktop:** mcpmux_* self-management meta tools with native approval ([8c740a5](https://github.com/crimsonsunset/mcp-mux/commit/8c740a5095a1c0ab5c435c4e634db045cddc2028))
* **gateway:** accept invoke_tool parameter aliases for agents ([f3544f3](https://github.com/crimsonsunset/mcp-mux/commit/f3544f3eb5803130c7f24d7802b33b94ef097668))
* **gateway:** add mcpmux_set_workspace_root and extend session keep-alive ([1e5d8db](https://github.com/crimsonsunset/mcp-mux/commit/1e5d8db83a1858e9b5efc2f8678f5cd708db2b4e))
* **gateway:** add search_tools inactive scan timing logs ([d1b9068](https://github.com/crimsonsunset/mcp-mux/commit/d1b9068635168b535860b19a382f676fef19075e))
* **gateway:** add SessionOverrideRegistry and list-path composition ([e72b64e](https://github.com/crimsonsunset/mcp-mux/commit/e72b64e96de49f2ba85637c8673e0942246aa59b))
* **gateway:** advertise lean meta-tool core in tools/list ([63f24a0](https://github.com/crimsonsunset/mcp-mux/commit/63f24a02a8b97f784573608f01a2518d663432a5))
* **gateway:** alias tool_arguments as invoke args key ([82768e5](https://github.com/crimsonsunset/mcp-mux/commit/82768e55099f0b0fc2040f36fe99e77406058372))
* **gateway:** enable remote MCP via public URL and tunnel consent ([598bb7a](https://github.com/crimsonsunset/mcp-mux/commit/598bb7af22b546873043d39630eddb8752fc2d89))
* **gateway:** FeatureSetResolver v2 + SessionRootsRegistry (shadow mode) ([76c6638](https://github.com/crimsonsunset/mcp-mux/commit/76c66386ca2876ed2e699c2ef8c1c65150bf3a5d))
* **gateway:** improve tool search with synonyms and inactive preview ([e428dcf](https://github.com/crimsonsunset/mcp-mux/commit/e428dcf576767707b4d6aab65f40ee08a8b6843b))
* **gateway:** invoke ergonomics — bare/qualified names and schema-lite search ([9532ce0](https://github.com/crimsonsunset/mcp-mux/commit/9532ce0023bdd0b8b17e9dd36df4431e70afc60e))
* **gateway:** lean meta core, server update policy, invoke UX, and module split ([4a71eba](https://github.com/crimsonsunset/mcp-mux/commit/4a71eba7f878fbcb94e4e3e4aced5aed75febfbb))
* **gateway:** Phase 1 — Lexical fix (token-overlap) ([c612af7](https://github.com/crimsonsunset/mcp-mux/commit/c612af71b471e2476ba19395b3135a7f3b074eec))
* **gateway:** Phase 1 — Server readiness model on list_servers ([36e62b3](https://github.com/crimsonsunset/mcp-mux/commit/36e62b3f08143cb7e5463f6a0326751eefb88649))
* **gateway:** Phase 2 — Embedding service + model lifecycle ([ab8cf8e](https://github.com/crimsonsunset/mcp-mux/commit/ab8cf8eb6d60ce13a1105e4d4b9d014e5bc9cb67))
* **gateway:** Phase 2 — Optional params + schema_complex in search ([fc2580e](https://github.com/crimsonsunset/mcp-mux/commit/fc2580ea3fc984d3674a26efe5c138ea8f60bafe))
* **gateway:** Phase 3 — Browse mode at scale ([6361ed2](https://github.com/crimsonsunset/mcp-mux/commit/6361ed2877b633ba0532be93be1c54b00083dbd8))
* **gateway:** Phase 3 — Hybrid fusion + embedding cache ([91d6942](https://github.com/crimsonsunset/mcp-mux/commit/91d6942e10eacaa4c85992f15841badada876c04))
* **gateway:** Phase 4 — Relevance eval + weight tuning ([e7e191e](https://github.com/crimsonsunset/mcp-mux/commit/e7e191e9d6b12b83df0b9aae5d20e0e7099be8e5))
* **gateway:** Phase 4 — Structured invoke denial ([636cd33](https://github.com/crimsonsunset/mcp-mux/commit/636cd33b954d380508eda6e5c6f85b6df28ed764))
* **gateway:** Phase 5 — invoke_example + opt-in preflight ([59c7ca3](https://github.com/crimsonsunset/mcp-mux/commit/59c7ca3280e39c87222c5a3092309b400a34d370))
* **gateway:** Phase A — Meta invoke core ([4978d70](https://github.com/crimsonsunset/mcp-mux/commit/4978d70d811c648e071a9ba798d255bf6c2683ae))
* **gateway:** Phase B — Result shaping on invoke ([4d11112](https://github.com/crimsonsunset/mcp-mux/commit/4d11112dcf5e7c91ccfc81b96e4416fc9ef48aac))
* **gateway:** Phase C — FeatureSet invoke ACL + surfaced tools ([270921d](https://github.com/crimsonsunset/mcp-mux/commit/270921d64c740a4932fa6dacd4a66b3c4833d4a9))
* **gateway:** Phase D — resource/prompt hard cut and disclosure meta tools ([2a31dc7](https://github.com/crimsonsunset/mcp-mux/commit/2a31dc7bdb7f7b21712ad2784c07c60ed2276236))
* **gateway:** surface prefilled params and meta-tool agent visibility ([de5ccb4](https://github.com/crimsonsunset/mcp-mux/commit/de5ccb40b4e7f5b5a3cf648339254c80e2981c64))
* **i18n:** migrate shared components deferred from Phase 1 ([97d94ea](https://github.com/crimsonsunset/mcp-mux/commit/97d94ea208cbec6fe4d0970cf3d9bb19fbd506c1))
* **i18n:** Phase 1 — Install, init, and nav/chrome layer ([60b2561](https://github.com/crimsonsunset/mcp-mux/commit/60b2561dda1ad838c8edcbe3816e18d0908b3a16))
* **i18n:** Phase 2 — Three heaviest feature pages ([7cea9fa](https://github.com/crimsonsunset/mcp-mux/commit/7cea9fa055e5011dde34eeee35121e292bebc9a5))
* **i18n:** Phase 3 — Remaining feature pages + string helpers ([930beab](https://github.com/crimsonsunset/mcp-mux/commit/930beab655f332d9bf8ca0080b6fdafd4579b85a))
* **i18n:** Phase 5 — Global shell & gateway confirms ([4d3b3a8](https://github.com/crimsonsunset/mcp-mux/commit/4d3b3a8ef3c55fb33d15f3b04a1d447b95ec164d))
* **i18n:** Phase 6 — Shared modals & registry CTAs ([3b78328](https://github.com/crimsonsunset/mcp-mux/commit/3b78328e3764f79f6e4e781c9731a2e77e328b67))
* **i18n:** Phase 7 — Settings, spaces, and servers tail ([e4fefa8](https://github.com/crimsonsunset/mcp-mux/commit/e4fefa86facbf2c0db39486ac60b0d723aaf0cbd))
* **i18n:** Phase 8 — Dashboard widgets & meta tools ([3ed380a](https://github.com/crimsonsunset/mcp-mux/commit/3ed380a0f42270674623249c5344cf5e9c43f741))
* **i18n:** Phase 9 — Audit, tests, and E2E gate ([dcea9fb](https://github.com/crimsonsunset/mcp-mux/commit/dcea9fb4cde13019415d90653fe80f142d89b0db))
* **i18n:** react-i18next full string extraction ([8a4c963](https://github.com/crimsonsunset/mcp-mux/commit/8a4c963b7ed859d1318fa40cec3f9ee79d385eb9))
* **meta-tools:** add mcpmux_list_servers read tool (Phase 2) ([f29543a](https://github.com/crimsonsunset/mcp-mux/commit/f29543aad281cff5cc8fbdd0feffdbb035824f0e))
* **meta-tools:** add session-scope enable/disable server tools (Phase 3) ([0905d38](https://github.com/crimsonsunset/mcp-mux/commit/0905d381e1982d0bd0c1bb97dc9514479fa33dab))
* **meta-tools:** add workspace-scope enable/disable server tools (Phase 4) ([b29d5e8](https://github.com/crimsonsunset/mcp-mux/commit/b29d5e8bbc3288952792c876cd5c141c1b5130a7))
* **meta-tools:** audit log + master switch + grants UI + E2E spec ([382c374](https://github.com/crimsonsunset/mcp-mux/commit/382c3747088b5df3392da3259c5dec69a9770328))
* **meta-tools:** Phase 1 — per-server default_params injection ([a92111c](https://github.com/crimsonsunset/mcp-mux/commit/a92111ccb61f1d5d047ca153f13202d023ef8d1c))
* **meta-tools:** Phase 1 — Thread dependencies into MetaToolContext ([c301b32](https://github.com/crimsonsunset/mcp-mux/commit/c301b327e21df9c5d088c4e3586530525a5c75f7))
* **meta-tools:** Phase 2 — Diagnostic helpers ([011f679](https://github.com/crimsonsunset/mcp-mux/commit/011f6790a5a8efb6a2a7dc0d57937e8a9691938d))
* **meta-tools:** Phase 3 — DiagnoseServerTool implementation ([31bb3d3](https://github.com/crimsonsunset/mcp-mux/commit/31bb3d35dc70a483e07a54e2eb69f733281e39ac))
* **meta-tools:** Phase 3 — required_params in search results ([84e9769](https://github.com/crimsonsunset/mcp-mux/commit/84e976901ef5ef3f918eaf266755e0a5fa62760d))
* **meta-tools:** Phase 4 — Tests + doc surface ([4c09e62](https://github.com/crimsonsunset/mcp-mux/commit/4c09e62cf01219fd8c78d04e782f44aa12709f4b))
* **meta-tools:** session override UI and settings toggle (Phase 5) ([b590b0b](https://github.com/crimsonsunset/mcp-mux/commit/b590b0b5ce5c4087ddb9e8e226caa26d94a8cba2))
* **routing:** per-client grants, multi-FS bindings, capability-branched resolver ([98bceb8](https://github.com/crimsonsunset/mcp-mux/commit/98bceb82a951261b68898d18de002731fb990413))
* **routing:** workspace-root-driven FeatureSet resolution ([5846a98](https://github.com/crimsonsunset/mcp-mux/commit/5846a98f1142e3e61c1fbdb7eb400b2087353ec1))
* **search-tools:** Phase 1 — Embedding repository + persistence ([2e05cad](https://github.com/crimsonsunset/mcp-mux/commit/2e05cad169d711e4fa862185d96733898e9a7650))
* **search-tools:** Phase 2 — Alias-free embedding text + spawn_blocking ([de4f616](https://github.com/crimsonsunset/mcp-mux/commit/de4f6162317b523ffb5cd295fd281aee33c245f8))
* **search-tools:** Phase 3 — Global store-backed search read path ([2736717](https://github.com/crimsonsunset/mcp-mux/commit/2736717c08a228eb28091ecb253eee42f0b6fd49))
* **search-tools:** Phase 4 — On-connect incremental warmer ([e07d6de](https://github.com/crimsonsunset/mcp-mux/commit/e07d6de6c4de6f102ce7bc9a9d50e68d588fd9ce))
* **search-tools:** Phase 5 — Observability, pruning & reconciliation ([d7076b2](https://github.com/crimsonsunset/mcp-mux/commit/d7076b22cd4e4ff9071ef975d2970f1fea34f95b))
* **server-clone:** Phase 1 — Core clone API + storage ([55d0758](https://github.com/crimsonsunset/mcp-mux/commit/55d0758568b87c6ec419901677d67877fc18940a))
* **server-clone:** Phase 2 — Clone wizard UI ([94decc8](https://github.com/crimsonsunset/mcp-mux/commit/94decc828ae5935351a6c0d9951e2ea41dafcbc4))
* **server-clone:** Phase 3 — Meta-tool + docs surfacing ([a45d34c](https://github.com/crimsonsunset/mcp-mux/commit/a45d34c10f8ffb1c338b48ce25d33aa3df4cc91c))
* **server-clone:** Phase 4 — Validation + edge cases ([c090948](https://github.com/crimsonsunset/mcp-mux/commit/c090948b111b64a0790cf29b34714c8a2c341e8b))
* **server-update:** explicit package updates and pending list UI ([822866c](https://github.com/crimsonsunset/mcp-mux/commit/822866c9450625cfa76bf44114ce73bfd20d3bf1))
* **server-update:** Phase 1 — Schema + Auto mode ([c20010b](https://github.com/crimsonsunset/mcp-mux/commit/c20010b9d0e9667595da3d99b1fad0a36a2fbe29))
* **server-update:** Phase 2 — Notify mode version probe + badge ([4316b1d](https://github.com/crimsonsunset/mcp-mux/commit/4316b1da3d2d61b0a555c97029ac90b6254fdd1e))
* **server-update:** Phase 3 — Pinned mode version lock ([73b184b](https://github.com/crimsonsunset/mcp-mux/commit/73b184b3d5000d91e2f76794869cb36085fb60be))
* **servers:** add per-install display name override ([6b70c0f](https://github.com/crimsonsunset/mcp-mux/commit/6b70c0f4e245ea90d5b0d988f8bb841b5586b4fa))
* **servers:** replace enable/disable buttons with labeled toggle ([2052bcc](https://github.com/crimsonsunset/mcp-mux/commit/2052bcca51c62fd3c5030341e0fab50064261c8b))
* **servers:** smarter OAuth startup and sticky My Servers toolbar ([cc50654](https://github.com/crimsonsunset/mcp-mux/commit/cc50654d4112a776fc9eda289083affab5b9437f))
* **settings:** build stamp UI and server update check feedback ([d88d976](https://github.com/crimsonsunset/mcp-mux/commit/d88d976c44df10793c253814d0b74ad6d5f64a7c))
* **ui:** add rename/edit for spaces, feature sets, and workspace bindings ([726492e](https://github.com/crimsonsunset/mcp-mux/commit/726492efc7775c49a8add0361c3e9703d6961734))
* **ui:** Contribute / Request affordances across the app ([c8c3f1c](https://github.com/crimsonsunset/mcp-mux/commit/c8c3f1c20035aac0b7a32644ac26ecf376d31361))
* **ui:** link dashboard stat cards to detail pages ([11814c4](https://github.com/crimsonsunset/mcp-mux/commit/11814c46354cd458194759c0583e17e7d2e82102))
* **ui:** premium Active FeatureSet + empty-state Clients page onboarding ([b4ba091](https://github.com/crimsonsunset/mcp-mux/commit/b4ba091aac69812b43d04447f1206372b4ad9607))
* web admin, backend facade, dashboard, and diagnose meta tool ([87f4fed](https://github.com/crimsonsunset/mcp-mux/commit/87f4fede13b5df6fd9d6303f00b03c12696dc078))
* **web-admin:** add formatted build stamp banner and server logs ([15b2112](https://github.com/crimsonsunset/mcp-mux/commit/15b2112b8006ae0b86942392aa70e9f27405f013))
* **web-admin:** detect stale SPA builds and warn in browser ([1874bc8](https://github.com/crimsonsunset/mcp-mux/commit/1874bc88893c347a92dd52e54c7423caa7fc8502))
* **web-admin:** Phase 1 — Parity inventory & test scaffolding ([90be2c1](https://github.com/crimsonsunset/mcp-mux/commit/90be2c1e2d4ce532128c475ff5bb3ea1c8431ed8))
* **web-admin:** Phase 2 — Admin server skeleton + CF Access gate ([8d43ab2](https://github.com/crimsonsunset/mcp-mux/commit/8d43ab2b28a69c9c7e382be0d30b05c3adb01b2b))
* **web-admin:** Phase 3 — command_bridge foundation (spaces pilot) ([f4d0d40](https://github.com/crimsonsunset/mcp-mux/commit/f4d0d409fde2a1c37f70f6f164b424f0dccccaa9))
* **web-admin:** Phase 4 — Transport layer + read-only REST API ([14c112e](https://github.com/crimsonsunset/mcp-mux/commit/14c112eab296480783d0f146756cc30e2a66b700))
* **web-admin:** Phase 5 — SSE event parity ([4f2989d](https://github.com/crimsonsunset/mcp-mux/commit/4f2989db13b59853a4d9483e999050ae88277006))
* **web-admin:** Phase 6 — Write API (config mutations) ([0e6f7e5](https://github.com/crimsonsunset/mcp-mux/commit/0e6f7e5c6f9c18100608d5a6fbabad23a133ec47))
* **web-admin:** Phase 7 — Web OAuth consent ([bfb8c0c](https://github.com/crimsonsunset/mcp-mux/commit/bfb8c0c4a745cf4690828d170f4a64ce3a65ff8d))
* **web-admin:** Phase 8 — Homelab integration + parity E2E + docs ([88973d4](https://github.com/crimsonsunset/mcp-mux/commit/88973d44ee56e0c3e778a119eab30d204219da73))
* **workspace:** scope bindings per OAuth client ([cbf42d7](https://github.com/crimsonsunset/mcp-mux/commit/cbf42d7d5c4a9d397db0f2d1c179fa165b6770c4))
* **workspaces:** Phase 1 — storage and domain for workspace icons ([acd48c0](https://github.com/crimsonsunset/mcp-mux/commit/acd48c098fdd1ab1a74ad453e6fb029bf3670c03))
* **workspaces:** Phase 2 — upload service and Tauri commands ([6c94d1c](https://github.com/crimsonsunset/mcp-mux/commit/6c94d1c70c075601d4196ecf454809080152b24c))
* **workspaces:** Phase 3 — frontend icon picker and rendering ([37dcc50](https://github.com/crimsonsunset/mcp-mux/commit/37dcc50d84307d3e16405148d81037168cee6eb5))


### Bug Fixes

* add Windsurf, JetBrains, and Android Studio to quick-connect grid ([#139](https://github.com/crimsonsunset/mcp-mux/issues/139)) ([fb58d9c](https://github.com/crimsonsunset/mcp-mux/commit/fb58d9ce6c46ec1a55356a9fecb35f34ae2b29f6))
* **admin-web:** unblock startup sync and stabilize admin E2E ([e8c573b](https://github.com/crimsonsunset/mcp-mux/commit/e8c573bed95936df704443bfa7a54283f4c58823))
* **admin:** cf access jwt validation actually works end-to-end ([d18afdc](https://github.com/crimsonsunset/mcp-mux/commit/d18afdc2437952a9334558f33fd78f40fe07d525))
* **ci:** gate updater signing and relax token budget guards ([fac5307](https://github.com/crimsonsunset/mcp-mux/commit/fac5307a313bbfd75329db0b13aa589247e06f8e))
* **ci:** rustfmt and align vitest with Tauri shell + useConfirm split ([610807a](https://github.com/crimsonsunset/mcp-mux/commit/610807ac86188a1b3ea595fba74b0d8e275302c5))
* **ConnectIDEs:** per-IDE correct instructions + popover opens upward ([c08d9fe](https://github.com/crimsonsunset/mcp-mux/commit/c08d9feefc171144772e9b25ff46f28c78f20a2b))
* **consent-model:** clippy sort_by_key in resolution sweep ([a8fc43c](https://github.com/crimsonsunset/mcp-mux/commit/a8fc43c14770b66de3af0a68a8b338200efb2e76))
* **consent-model:** sync meta-tool approval dismiss across surfaces ([32908c0](https://github.com/crimsonsunset/mcp-mux/commit/32908c07fd07d910d6a7d4af5735349db164fbd4))
* **desktop:** gate debug-only keys import for release clippy ([ea333dd](https://github.com/crimsonsunset/mcp-mux/commit/ea333ddef9528ff8c0147c4e7ea2fcde55ed5965))
* **desktop:** Phase 1 — Broken invokes + dead exports ([a09d487](https://github.com/crimsonsunset/mcp-mux/commit/a09d4873bd98ab0b5162e4bb594ed025e02f1c5a))
* **desktop:** Phase 2 — Server lifecycle call-site consolidation ([76fb0d7](https://github.com/crimsonsunset/mcp-mux/commit/76fb0d7a0fa1faddfa67d5e741555a2cf2b2881f))
* **desktop:** Phase 3 — API layer consolidation ([3790f84](https://github.com/crimsonsunset/mcp-mux/commit/3790f84b45b96104c33f52e10044047e78181cbe))
* **desktop:** Phase 4 — Event channel audit + hook alignment ([e05604f](https://github.com/crimsonsunset/mcp-mux/commit/e05604f8b2d844d4553a36f2da922c24f394fa93))
* **desktop:** pre-web-admin IPC and event cleanup ([bc718dd](https://github.com/crimsonsunset/mcp-mux/commit/bc718dd6bd059351ef64abe37bc2d300d4a7cfc7))
* **desktop:** restore custom titlebar window dragging ([308963b](https://github.com/crimsonsunset/mcp-mux/commit/308963b64f69b1b098ed0d769f4b8ce3dd301f44))
* **desktop:** self-host Monaco so Add custom server works in prod ([45da4e0](https://github.com/crimsonsunset/mcp-mux/commit/45da4e0e85e4bdec4a00cfd31c7ad18ad030c490))
* **desktop:** show build stamp console banner in all prod builds ([ec02790](https://github.com/crimsonsunset/mcp-mux/commit/ec02790a7e3a5de203aaf3d4afa60d19f477a08d))
* **gateway,ui:** collapse describe tools, fire list_changed on Connect, badge denominator ([02bd7b9](https://github.com/crimsonsunset/mcp-mux/commit/02bd7b9935bf6bd3e8c93feb1bf17d9bbb0e6617))
* **gateway:** address PR [#3](https://github.com/crimsonsunset/mcp-mux/issues/3) review — cold-start warmer, O(N) hybrid, mutex ([8a1f4cd](https://github.com/crimsonsunset/mcp-mux/commit/8a1f4cd5b11b4c76c4f5c6845029b14ce13cd460))
* **gateway:** alias params→args in invoke_tool, scope:all in search_tools, renderNow tautology ([da79edb](https://github.com/crimsonsunset/mcp-mux/commit/da79edbd3ec06a4aed3744ece9eea5991ed9b64f))
* **gateway:** allow direct call_tool for surfaced backend tools ([22d92c3](https://github.com/crimsonsunset/mcp-mux/commit/22d92c386ab77427085f104cf01821a63f42d91f))
* **gateway:** apply session overrides in call_tool authorization ([5269a18](https://github.com/crimsonsunset/mcp-mux/commit/5269a189085457d041b678c18b560ac1b50d5441))
* **gateway:** coalesce JSON/YAML invoke payloads before filter shaping ([bf1555c](https://github.com/crimsonsunset/mcp-mux/commit/bf1555c19d371d28f22334cfb35090713f8955a5))
* **gateway:** complete remote MCP OAuth path and CF service-token dev UX ([3824d9a](https://github.com/crimsonsunset/mcp-mux/commit/3824d9a87080fc5e271eeb7d364b6f182cc1fef5))
* **gateway:** drop mcpmux_describe_workspace meta tool ([42bfbf9](https://github.com/crimsonsunset/mcp-mux/commit/42bfbf984f9673256cd138a034c007d52797260f))
* **gateway:** forward structuredContent in proxied tool results ([d519a79](https://github.com/crimsonsunset/mcp-mux/commit/d519a792857d3d26b005560cedc91e9e826e5c74))
* **gateway:** GAIT QA meta invoke ACL, filter shaping, and QA tracker ([1fbea1f](https://github.com/crimsonsunset/mcp-mux/commit/1fbea1f11d2bec46539de8948a7df5d7517a3dbd))
* **gateway:** get_tool_schema resolves bare names via feature_name ([7e1ab44](https://github.com/crimsonsunset/mcp-mux/commit/7e1ab448a66aa9402cf177db846a859fb4790eb9))
* **gateway:** harden invoke preflight/truncation and measure meta-tool token budget ([4585ce8](https://github.com/crimsonsunset/mcp-mux/commit/4585ce801f9d2893621ffc1666dd13e971523b8c))
* **gateway:** improve meta-tool DX for ACL, schema batch, and max_bytes ([85113e7](https://github.com/crimsonsunset/mcp-mux/commit/85113e7e6a95095ba6aeac2c35483d69f7adf498))
* **gateway:** make invoke result truncation opt-in via filter ([433e7bd](https://github.com/crimsonsunset/mcp-mux/commit/433e7bdbf5b7fd860ea7fc350f15b36ef16d42bf))
* **gateway:** persist probed current_version for bare npx/uvx badging ([c489692](https://github.com/crimsonsunset/mcp-mux/commit/c4896926a84eea44c5e82be9d137ed611e1af7a3))
* **gateway:** Phase 0 — eager embedding model init on warm ([7cd47b0](https://github.com/crimsonsunset/mcp-mux/commit/7cd47b05ff53e390810e23eddb9bd7414e95141e))
* **gateway:** Phase 2 — npx notify correctness + cache-bust ([6c3fa33](https://github.com/crimsonsunset/mcp-mux/commit/6c3fa33e40590985113abf8b36b99cd6c43877e4))
* **gateway:** Phase 3 — uvx PyPI probe correctness ([e4562ba](https://github.com/crimsonsunset/mcp-mux/commit/e4562ba7f1c37375577ffbbf2870ad4fa42548cd))
* **gateway:** Phase 6 — Root-race fix ([4195944](https://github.com/crimsonsunset/mcp-mux/commit/4195944c4dfbdca7ff64285911293fc1283411f8))
* **gateway:** PR review fixes — invoke normalize, tests, doc refresh ([8df4252](https://github.com/crimsonsunset/mcp-mux/commit/8df4252ab0b2f6784adcd7ac716c43130ac45ba0))
* **gateway:** PR review fixes for invoke readiness and browse docs ([2ae904e](https://github.com/crimsonsunset/mcp-mux/commit/2ae904eef3cac916a86998195565a778f697e037))
* **gateway:** resolve read_resource via grant-scoped clone routing ([a4a212a](https://github.com/crimsonsunset/mcp-mux/commit/a4a212a6d9bf1bdd843fe85ebcfe4a3f48ce222c))
* **gateway:** route to resolved space + accept URL client_ids in meta tools ([c2f02f6](https://github.com/crimsonsunset/mcp-mux/commit/c2f02f6eba368d564a4c3ead3e91e5dee016fe50))
* **gateway:** rustfmt CI drift and harden shell PATH probe ([b055836](https://github.com/crimsonsunset/mcp-mux/commit/b055836fc22625e2d503134a697a2b37c21cc7ba))
* **gateway:** skip approval when bind target is already bound ([977bbee](https://github.com/crimsonsunset/mcp-mux/commit/977bbee65807bd5f918f1514a5f397f3f3ca22bf))
* **gateway:** strip PEP 508 version pin before uvx PyPI lookup ([92e340f](https://github.com/crimsonsunset/mcp-mux/commit/92e340fbec9b80ab91e4e19c50f5a714e5ced452))
* **gateway:** unblock warmer embeds and document O0b QA ([b68f672](https://github.com/crimsonsunset/mcp-mux/commit/b68f672de5fac571fc42f657469bef038526db88))
* **gateway:** upsert workspace binding in bind_current_workspace ([ed8a5fb](https://github.com/crimsonsunset/mcp-mux/commit/ed8a5fbbc9323fffdac16ecacf1833a74b11bb2d))
* **gateway:** wire approval publisher on auto-start + focus window on popup ([ac33136](https://github.com/crimsonsunset/mcp-mux/commit/ac3313653c083c8ea060f25137d41f41eedc4b66))
* **handler:** single-flight on-demand probe per session ([deab680](https://github.com/crimsonsunset/mcp-mux/commit/deab6807e26fdeae2a127a3544e9407bc63a786f))
* **i18n:** vitest i18n harness, cancelLabel, and CI blockers ([22da151](https://github.com/crimsonsunset/mcp-mux/commit/22da151bd09e475a458ddb6477f05261cce34880))
* **lint:** resolve set-state-in-effect errors blocking lint sweep ([c236a06](https://github.com/crimsonsunset/mcp-mux/commit/c236a067803b3a6c39cbea27312abed49a0931c3))
* **macos:** hide Dock icon when McpMux runs tray-only ([10aea0d](https://github.com/crimsonsunset/mcp-mux/commit/10aea0d3871a82d6863fa12469fca60e073dc1b8))
* **macos:** register McpMux with TCC for child MCP server permissions ([8243494](https://github.com/crimsonsunset/mcp-mux/commit/8243494431c05de3149dfe02745c8e1e8d2b4f01))
* **meta-tools:** include installed servers with no tool features in list_servers ([30ede00](https://github.com/crimsonsunset/mcp-mux/commit/30ede00141788b63ba092ffb51f652fcdc1184d3))
* **meta-tools:** Phase 2 — bare-name invoke suggestions ([1d79cfb](https://github.com/crimsonsunset/mcp-mux/commit/1d79cfb1738b093f648fe2501db5d73d4f894cab))
* **meta-tools:** scope to caller's resolved Space + drop stale pin/active wording ([a313ffd](https://github.com/crimsonsunset/mcp-mux/commit/a313ffd0cc27c2d657972c185b2b23af0cc0c1d2))
* **notifier:** lazy-GC dead sessions on every fanout / per-peer push ([c9a18b7](https://github.com/crimsonsunset/mcp-mux/commit/c9a18b7e88f6bc97e51e6d9588b33d1dd0beb3b1))
* **notifier:** tag every list_changed push with session_id + client_id ([a83e288](https://github.com/crimsonsunset/mcp-mux/commit/a83e28876556003930ff93cfd6e9a40b848b3703))
* **oauth,services:** port [#152](https://github.com/crimsonsunset/mcp-mux/issues/152) — DCR redirect URI tolerance + clippy ([985e6a4](https://github.com/crimsonsunset/mcp-mux/commit/985e6a4d0c025ebdea435f6884e91158e31d0ce9))
* **oauth:** annotate rand gen for Windows type inference (E0283) ([ed7543a](https://github.com/crimsonsunset/mcp-mux/commit/ed7543a50cbbe333da06ca912491e917cef1c353))
* **oauth:** drop duplicate RFC 8707 resource param ([f08e8ec](https://github.com/crimsonsunset/mcp-mux/commit/f08e8ec1c18024f605199ce799338ca0d5c1adf1))
* resolve validate blockers — lint error, type error, and failing test ([35b1427](https://github.com/crimsonsunset/mcp-mux/commit/35b142701c107a74d993c2019d9c95b4a12b73ad))
* **routing:** close root-fetch race + Starter editability + binding autosave ([af600c8](https://github.com/crimsonsunset/mcp-mux/commit/af600c859ca3f8b35ef61c5cbd05b2f7cef18292))
* **storage:** gate file-key migration to non-Windows for CI compile ([adf8b53](https://github.com/crimsonsunset/mcp-mux/commit/adf8b53768ee24b268b773969de282ce2e52741a))
* **storage:** import MasterKeyProvider for file-key migration trait methods ([d30eea8](https://github.com/crimsonsunset/mcp-mux/commit/d30eea8a8228c6f0070d236396d1833a6d1f99d9))
* **storage:** prevent split-key encryption on keychain unlock failure ([3da8277](https://github.com/crimsonsunset/mcp-mux/commit/3da8277bc2003c5a0d04e4916153e042203a64f7))
* **storage:** register migration 022; document default_params ([568a89e](https://github.com/crimsonsunset/mcp-mux/commit/568a89e69fa747d68478ce0bd8b8fcb3ae8eed73))
* **tauri:** gate macOS-only tracing::warn import for clippy on Linux CI ([5888e02](https://github.com/crimsonsunset/mcp-mux/commit/5888e023ba8cce32cd428ac1a5d5b90ae6b4a962))
* **test:** use tokio RwLock in admin_api gateway harness ([aedbbe4](https://github.com/crimsonsunset/mcp-mux/commit/aedbbe4154ffe5ba7f5daa2e0aa7a9178b4906c3))
* **ui:** Clients onboarding — single generic 3-step card, always shown ([7dcd554](https://github.com/crimsonsunset/mcp-mux/commit/7dcd55485333dd6c2d21a113742c5bac2f1959a2))
* **ui:** split one-click vs copy-paste install in Clients onboarding ([ce1d384](https://github.com/crimsonsunset/mcp-mux/commit/ce1d3845fa2ac44d69a2ccdc7886669be6b4e3fb))
* **updates:** clear stale version badges after explicit package update ([d0d0232](https://github.com/crimsonsunset/mcp-mux/commit/d0d0232e7d8b327c3f9157f5f25860fbc09463b3))
* **web-admin:** CSRF startup race and workspace icon HTTP serve ([2de65fd](https://github.com/crimsonsunset/mcp-mux/commit/2de65fd380c3c1b6f56d6739c49d257462592113))
* **web-admin:** desktop-shell boundary and browser-safe frontend ([acf3f92](https://github.com/crimsonsunset/mcp-mux/commit/acf3f92e287045651b7ef316402d0b6bfdff5234))
* **web-admin:** harden admin server from PR [#2](https://github.com/crimsonsunset/mcp-mux/issues/2) review findings ([0c1a017](https://github.com/crimsonsunset/mcp-mux/commit/0c1a017aa6615bbea249e92157707203c2e16f8a))
* **web-admin:** restore server connection statuses in browser UI ([cd32282](https://github.com/crimsonsunset/mcp-mux/commit/cd32282287ccecce2290e28cf31276a2f9889523))
* **workspaces:** render uploaded workspace icons reliably ([7683137](https://github.com/crimsonsunset/mcp-mux/commit/76831371860211f7d32f9800981f94716102859f))


### Performance

* **gateway:** dedupe resolver resolve in search + read-path diagnostics ([17584c6](https://github.com/crimsonsunset/mcp-mux/commit/17584c61dda0d640cb8991092133416fc9c50b11))
* **gateway:** Phase 7 — Inactive scan SQL rewrite ([494c693](https://github.com/crimsonsunset/mcp-mux/commit/494c693b2803780ad6965ba950af63a21222038d))
* **gateway:** Phase 8 — Per-session active index cache ([16d5fff](https://github.com/crimsonsunset/mcp-mux/commit/16d5fffbc75703f11db02cf4654cf8bd1028db46))
* **gateway:** precompute corpus stats for O(N) lexical rank ([5ad6a97](https://github.com/crimsonsunset/mcp-mux/commit/5ad6a97750ec390cff22f871211aae98112a6622))


### Refactoring

* **dev:** extract dev-kill helpers and document pnpm dev:stop ([b6babdd](https://github.com/crimsonsunset/mcp-mux/commit/b6babdd78b712c4d3fa5640b3d299fc938bc07a2))
* **gateway:** Phase 4 — guards, async hygiene, headless, auto-exclusion ([7414f75](https://github.com/crimsonsunset/mcp-mux/commit/7414f75d9201ef270c838ed0d9852a69c55fd49e))
* **gateway:** remove temporary [embed] diag warns ([c359e32](https://github.com/crimsonsunset/mcp-mux/commit/c359e32a3f6d0fdd168398c252991eefed17d657))
* **gateway:** Wave 3 — Phases 5, 6 (parallel split) ([20b87e0](https://github.com/crimsonsunset/mcp-mux/commit/20b87e05113a73c75880635ce2be2fab89abf63e))
* **meta_tools:** Wave 1 — Phases 1, 2, 3 (parallel split) ([a8bb6f4](https://github.com/crimsonsunset/mcp-mux/commit/a8bb6f48070a6ec84d3eebbfbf7ac022f49d0429))
* **meta_tools:** Wave 4 — Phase 7 search_tools_index extract ([3ab4187](https://github.com/crimsonsunset/mcp-mux/commit/3ab418773f6ce00024364f639052b6d0a98b8343))
* **meta_tools:** Wave 5 — Phases 8, 9 (parallel split) ([be00a28](https://github.com/crimsonsunset/mcp-mux/commit/be00a28d9c685a61bf4ba7221511f9ecf796b205))
* **meta-tools:** Phase 1 — Extract meta_tool_common.rs ([50be85b](https://github.com/crimsonsunset/mcp-mux/commit/50be85b50eec4716780969752069932ce5a034e3))
* **meta-tools:** Phase 2 — Split search_tools.rs + list_servers.rs ([a0914b3](https://github.com/crimsonsunset/mcp-mux/commit/a0914b35a9614c0007ec90b1d5ff23f8cd427fe1))
* **meta-tools:** Phase 3 — Split remaining tools, delete tools.rs ([aeab6c8](https://github.com/crimsonsunset/mcp-mux/commit/aeab6c8d85c51282651094dbe731a6d7db396d3c))
* **meta-tools:** Phase 4 — Split invoke.rs ([77aaac2](https://github.com/crimsonsunset/mcp-mux/commit/77aaac2f24cd76db37f32b2cdb4b71998fdbd7bc))
* **meta-tools:** Phase 5 — Verify + doc touch ([bbb9cca](https://github.com/crimsonsunset/mcp-mux/commit/bbb9ccadb11a8c1c9084134d6ed2cf5b61492cd6))
* **web-admin:** split fetch routes and add live gateway tests ([558a319](https://github.com/crimsonsunset/mcp-mux/commit/558a319a19667c8cf8779bd89dfb3607e28037ee))


### Documentation

* add macOS build-from-source and app swap guide ([5e04746](https://github.com/crimsonsunset/mcp-mux/commit/5e04746a5fdd34304cb1e77993c4fc6e54303be0))
* add O-verify fix-battery section to QA runbook ([3c7c890](https://github.com/crimsonsunset/mcp-mux/commit/3c7c89093e72c3c9621d330155f8c66775e42b22))
* add remaining fork planning docs for app features ([595868f](https://github.com/crimsonsunset/mcp-mux/commit/595868f2912e69234dd756dd34910c8451d0331d))
* add remote access guide for tunnel and CF Access setup ([8f23a41](https://github.com/crimsonsunset/mcp-mux/commit/8f23a41732e332ace799bc5a8a1d800d4344f767))
* add unified backend facade (Option 4A) planning ([d4fd0a5](https://github.com/crimsonsunset/mcp-mux/commit/d4fd0a594805f921693f8370360cc27b9cbc6624))
* **backend:** Phase 5 — Facade cleanup + autonomous decisions ([2267884](https://github.com/crimsonsunset/mcp-mux/commit/2267884895673364f265ade172e79b1b9bccd7ce))
* backfill hybrid K-N from G2 and complete O2-O4 runbook sign-off ([0e87cd8](https://github.com/crimsonsunset/mcp-mux/commit/0e87cd857ba66183703b3e725f72c97d77111d89))
* **desktop:** Phase 5 — Verification + web admin unblock ([9daadea](https://github.com/crimsonsunset/mcp-mux/commit/9daadea016b9a4cb43cc4d026289dba18f1c62d5))
* **docs:** Phase 2 — backend index and core synthesis ([157ffd8](https://github.com/crimsonsunset/mcp-mux/commit/157ffd8541fb1a178eaf180673e9a545fec4de37))
* **docs:** Phase 3 — remaining backend technical and guides ([cbb7d44](https://github.com/crimsonsunset/mcp-mux/commit/cbb7d442228732a7a90b5b25735c86754589e6be))
* **docs:** Phase 4 — frontend domain and cross-cutting wiring ([181d1a1](https://github.com/crimsonsunset/mcp-mux/commit/181d1a1b9a8bf9f7febc499588d061a7f4b99879))
* **guides:** document password-free prod build-and-swap loop ([0567eef](https://github.com/crimsonsunset/mcp-mux/commit/0567eeff4058b93e0db377d6094f5dc8d9b2cb6e))
* lean meta surface and invoke ergonomics — agent-validated ([02a028b](https://github.com/crimsonsunset/mcp-mux/commit/02a028bfb39470790db5f2315a3f415cec8c4caa))
* mark fork PR CI verification complete ([7876b12](https://github.com/crimsonsunset/mcp-mux/commit/7876b1238513eaba663155909c635a974a2e1daa))
* mark fork PR CI verification complete ([40d656f](https://github.com/crimsonsunset/mcp-mux/commit/40d656f3e3ddfb859dec22c8e0ddd380c58a0f21))
* **meta_tools:** Wave 2 — Phase 4 verify + maintainer doc touch ([9c70972](https://github.com/crimsonsunset/mcp-mux/commit/9c70972033932a5d39cd34cc130c16999e16e8f9))
* **meta_tools:** Wave 6 — Phase 10 final verify + doc touch ([172012a](https://github.com/crimsonsunset/mcp-mux/commit/172012aafb473955ed9b0c56c685946a2cd306e5))
* **meta-tools:** close Phase 5 with README and planning reconciliation ([5730b25](https://github.com/crimsonsunset/mcp-mux/commit/5730b25b518a08b9c6d7ad1e8b99d5d7c7caa4d5))
* plan meta-tool agent UX round 3 (readiness, browse, structured invoke) ([1fc6ce4](https://github.com/crimsonsunset/mcp-mux/commit/1fc6ce4294f185f747133ea62be0b089590ec7c3))
* plan meta-tools module split (tools.rs + invoke.rs co-extract) ([b6962ce](https://github.com/crimsonsunset/mcp-mux/commit/b6962ce5d4c7a76402002a2ba6fe1218ee092180))
* **planning:** add feature-set consent model plan ([88e33a4](https://github.com/crimsonsunset/mcp-mux/commit/88e33a498ec6fdf61188044527ad83cc43f72570))
* **planning:** add hybrid ranking QA and reconcile shipped plans ([d6d9df5](https://github.com/crimsonsunset/mcp-mux/commit/d6d9df50bdd9b9e8e3f2b36630b38dd8456d5d7f))
* **planning:** add meta-gateway invoke manual QA runbook ([993f378](https://github.com/crimsonsunset/mcp-mux/commit/993f378324ace3a2843ed9c6385e64f0323a5a9d))
* **planning:** add meta-gateway invoke model for agent tool discovery ([f767e79](https://github.com/crimsonsunset/mcp-mux/commit/f767e7923bd86de5bc1c99ff8ccbd47b768ecc8d))
* **planning:** add persistent embedding cache plan and QA ([ed48359](https://github.com/crimsonsunset/mcp-mux/commit/ed483597321508e0caa63ecb81d32be08fe242f1))
* **planning:** add plan for dynamic MCP toggle meta tools ([7bf4cb1](https://github.com/crimsonsunset/mcp-mux/commit/7bf4cb17548bf20daa1a49d07565377ceef98001))
* **planning:** add search_tools hybrid semantic ranking plan ([f1c3908](https://github.com/crimsonsunset/mcp-mux/commit/f1c3908820bdee8e3f2668960eeb4df3e998a787))
* **planning:** add server account clones implementation plan ([7d762a8](https://github.com/crimsonsunset/mcp-mux/commit/7d762a81b9b4841555f4e6b98d84535e2ccb0048))
* **planning:** add server update policy ([ce819a0](https://github.com/crimsonsunset/mcp-mux/commit/ce819a04036efd83fee8f659dbd7cb9a65cb2ae0))
* **planning:** add server update policy audit and remediation plan ([1f81548](https://github.com/crimsonsunset/mcp-mux/commit/1f81548c77903019737a9ff2a2d231ee9b24c791))
* **planning:** add web admin parity matrix and pre-cleanup gate ([1332a63](https://github.com/crimsonsunset/mcp-mux/commit/1332a63d0aa5994bf9ad1e6d17f15bff93cf232a))
* **planning:** add workspace binding icons plan ([cedfbce](https://github.com/crimsonsunset/mcp-mux/commit/cedfbcec3445fc2594e99ffb3cc799525aa81c1c))
* **planning:** canonical fork branch is dev ([419f527](https://github.com/crimsonsunset/mcp-mux/commit/419f527b52a21156ce4d4c7ed8e447d6acbf004f))
* **planning:** complete meta-gateway invoke manual QA sign-off ([5508c5e](https://github.com/crimsonsunset/mcp-mux/commit/5508c5e0032cced1c47e9bca7ff8b7d379f73ecb))
* **planning:** fork integration guide and session readiness plans ([b9d2370](https://github.com/crimsonsunset/mcp-mux/commit/b9d237020194a9551f95c53e89b95a250d84b368))
* **planning:** Phase 4 — reconcile workspace binding icons plan ([de5df79](https://github.com/crimsonsunset/mcp-mux/commit/de5df7904f0566e9a84da21a5f193e6c090b9f28))
* **planning:** reconcile feature-set consent model plan ([bc6783e](https://github.com/crimsonsunset/mcp-mux/commit/bc6783ee617c4519ffe8fc7463be1e04e00474bb))
* **planning:** reconcile mcpmux_diagnose_server plan with as-built state ([e4f420e](https://github.com/crimsonsunset/mcp-mux/commit/e4f420e99e6b05e361d97c20057b04fce2833830))
* **planning:** reconcile meta-gateway invoke QA progress ([c505b07](https://github.com/crimsonsunset/mcp-mux/commit/c505b0755c80fa45b01163531c86e6faec6d1e49))
* **planning:** reconcile meta-gateway Phase D QA sign-off ([0bab392](https://github.com/crimsonsunset/mcp-mux/commit/0bab392c168727def73a199637b0260d5c41080c))
* **planning:** reconcile pr-2 review with cf access smoke findings ([b13f135](https://github.com/crimsonsunset/mcp-mux/commit/b13f1351a5ca4ca079d625859d5464f0d09e6d13))
* **planning:** reconcile web admin docs with post-review state ([0fadfc7](https://github.com/crimsonsunset/mcp-mux/commit/0fadfc78bfc39d2755cd4e0791492cbc7e8ef07e))
* **planning:** record Phase 5 QA pass and PR [#155](https://github.com/crimsonsunset/mcp-mux/issues/155) ([01b60e3](https://github.com/crimsonsunset/mcp-mux/commit/01b60e3fc496645623633663fd175d1afad50360))
* **planning:** sign off meta-gateway invoke targeted retest ([b7626fb](https://github.com/crimsonsunset/mcp-mux/commit/b7626fb3ec41af1c03b2ab0f6bbd4e7316766721))
* **planning:** sync meta-gateway invoke status with QA sign-off ([6884742](https://github.com/crimsonsunset/mcp-mux/commit/6884742b23027fe7833077bda7f00f43628bd38a))
* **planning:** tool-level session pin meta tool ([d3940bb](https://github.com/crimsonsunset/mcp-mux/commit/d3940bbe9bcdf45ee0996d98c089aac69c081d99))
* **planning:** web admin mode for remote UI access ([9e4c3cd](https://github.com/crimsonsunset/mcp-mux/commit/9e4c3cd88ffd27a537173b281dc2519b65c7abf2))
* reconcile operator and planning docs with backend facade ([0a73503](https://github.com/crimsonsunset/mcp-mux/commit/0a7350390ac2e93fdf831925e9415e6e09139307))
* record O-verify and O1 QA results in runbook ([e90b5e7](https://github.com/crimsonsunset/mcp-mux/commit/e90b5e78bd6879a7d079d08b3665b8e1a1619711))
* remove personal fork planning docs from upstream tree ([3a892c6](https://github.com/crimsonsunset/mcp-mux/commit/3a892c6ee90095c75b61d242f226669afe79c24e))
* rename build-from-source-macos.md to run-from-source-macos.md ([da91fd9](https://github.com/crimsonsunset/mcp-mux/commit/da91fd969213b35df2c74bd2f0cfe24352bba69a))
* replace personal hostnames and fork URLs with generic placeholders ([3a04aaa](https://github.com/crimsonsunset/mcp-mux/commit/3a04aaa0437bcb8b7bf20e68c5b21d9d61b4eb18))
* restore sanitized planning docs with homelab refs redacted ([7625fda](https://github.com/crimsonsunset/mcp-mux/commit/7625fda93d61f6b467163e2f32a5840e9fd8f2c8))
* **run-from-source:** document pnpm dev watch flow ([2edcd29](https://github.com/crimsonsunset/mcp-mux/commit/2edcd298b0df94fc06a71c1f0700f015cafb60b2))
* search read-path plan + consent-model QA runbook updates ([04c3adc](https://github.com/crimsonsunset/mcp-mux/commit/04c3adcc2a1a27cd23d29bb5b10ec1aad33c5036))
* **server-update-policy:** Phase 1 — Audit & baseline ([f06e6a7](https://github.com/crimsonsunset/mcp-mux/commit/f06e6a7f6746ebe39783fffe9ea03cdeba02506e))
* **testing:** add CI strategy doc and index it in testing README ([f336907](https://github.com/crimsonsunset/mcp-mux/commit/f3369074f37360848d3262a757fd2cfac22b884b))
* update get_tool_schema bare-name resolution across reference docs ([d741785](https://github.com/crimsonsunset/mcp-mux/commit/d741785f804d22eaa18d262c431c6daeb9b467d2))
* update meta-tool surface count and lean-core status ([c343ff4](https://github.com/crimsonsunset/mcp-mux/commit/c343ff473e985672c4020dd577a0add61b60b02f))
* **workspace:** add onboarding guide and fix E2E getActiveSpace helper ([616e614](https://github.com/crimsonsunset/mcp-mux/commit/616e6143e4199d0ad80cbfbfcfbe2fee224cdeec))

## [0.3.0](https://github.com/mcpmux/mcp-mux/compare/v0.2.3...v0.3.0) (2026-02-25)


### Features

* post-action UX guidance, ConfirmDialog, and client auto-select ([#136](https://github.com/mcpmux/mcp-mux/issues/136)) ([44d934c](https://github.com/mcpmux/mcp-mux/commit/44d934c678c4d7a2eebc996928e2fb37c07d7a8e))

## [0.2.3](https://github.com/mcpmux/mcp-mux/compare/v0.2.2...v0.2.3) (2026-02-21)


### Bug Fixes

* allow process restart after update and detect Homebrew version mismatch ([#134](https://github.com/mcpmux/mcp-mux/issues/134)) ([ecdbaca](https://github.com/mcpmux/mcp-mux/commit/ecdbacafaff573f497ce6db8614fa39993a28a32))
* debounce analytics search tracking to capture final query ([#132](https://github.com/mcpmux/mcp-mux/issues/132)) ([0f17ddb](https://github.com/mcpmux/mcp-mux/commit/0f17ddb768b5d309a3a73cc6df492f656e205f69))

## [0.2.2](https://github.com/mcpmux/mcp-mux/compare/v0.2.1...v0.2.2) (2026-02-20)


### Bug Fixes

* detect OAuth requirement from unexpected content-type responses ([#128](https://github.com/mcpmux/mcp-mux/issues/128)) ([d894d17](https://github.com/mcpmux/mcp-mux/commit/d894d17c7c4c5841b7eb39dc1d7068dbcb447656))
* wire up HTTP definition headers orthogonally from auth ([#125](https://github.com/mcpmux/mcp-mux/issues/125)) ([04380e0](https://github.com/mcpmux/mcp-mux/commit/04380e0979ab428351185d381001d209e6a4993b))


### Documentation

* add user guide with screenshots ([#130](https://github.com/mcpmux/mcp-mux/issues/130)) ([a97a133](https://github.com/mcpmux/mcp-mux/commit/a97a1333520fc1ac54f061344970cf493807ca87))
* add user guide with screenshots ([#131](https://github.com/mcpmux/mcp-mux/issues/131)) ([ee28e8b](https://github.com/mcpmux/mcp-mux/commit/ee28e8be432d2b1532f3f98067ba9004c4a18374))

## [0.2.1](https://github.com/mcpmux/mcp-mux/compare/v0.2.0...v0.2.1) (2026-02-19)


### Bug Fixes

* regenerate ICO with proper sizes & increase connection timeout ([#123](https://github.com/mcpmux/mcp-mux/issues/123)) ([2d88b25](https://github.com/mcpmux/mcp-mux/commit/2d88b259e9ca1bbc1ac57405854d732d8437cce3))


### Refactoring

* remove Password and Textarea from InputType enum ([#122](https://github.com/mcpmux/mcp-mux/issues/122)) ([bd06386](https://github.com/mcpmux/mcp-mux/commit/bd06386e04020da381135761a631ab38543ae414))

## [0.2.0](https://github.com/mcpmux/mcp-mux/compare/v0.1.2...v0.2.0) (2026-02-18)


### Features

* add select, file_path, and directory_path input types ([#121](https://github.com/mcpmux/mcp-mux/issues/121)) ([942ee1a](https://github.com/mcpmux/mcp-mux/commit/942ee1ae88f60aa1454bc97cec3839bcacf74454))


### Bug Fixes

* add one-click IDE install for VS Code and Cursor ([#119](https://github.com/mcpmux/mcp-mux/issues/119)) ([5b280fb](https://github.com/mcpmux/mcp-mux/commit/5b280fbfdcd04165827b7662ba6896cea96deb83))
* version display & update check ([#117](https://github.com/mcpmux/mcp-mux/issues/117)) ([b40c59b](https://github.com/mcpmux/mcp-mux/commit/b40c59bfb7b9ec19be8848abe04e38ba6fed1422))

## [0.1.2](https://github.com/mcpmux/mcp-mux/compare/v0.1.1...v0.1.2) (2026-02-18)


### Bug Fixes

* resolve npx/node PATH on macOS GUI apps ([#113](https://github.com/mcpmux/mcp-mux/issues/113)) ([98c013d](https://github.com/mcpmux/mcp-mux/commit/98c013d4e6955e678949df6068c038e1b8cf00fc))


### Documentation

* improve README first impression with problem/fix diagrams ([#109](https://github.com/mcpmux/mcp-mux/issues/109)) ([b15482b](https://github.com/mcpmux/mcp-mux/commit/b15482b32a016e3ca92753f26212f5827f744903))

## [0.1.1](https://github.com/mcpmux/mcp-mux/compare/v0.1.0...v0.1.1) (2026-02-16)


### Bug Fixes

* file-based keychain fallback for headless Linux/WSL ([#103](https://github.com/mcpmux/mcp-mux/issues/103)) ([9b60e0b](https://github.com/mcpmux/mcp-mux/commit/9b60e0bbe47a2318e7352efd3ba8b1888f393f38))
* stdio enable error UI state ([#104](https://github.com/mcpmux/mcp-mux/issues/104)) ([b4598e6](https://github.com/mcpmux/mcp-mux/commit/b4598e60e12d3389717fc2252bac8eb29e96f9c9))

## [0.1.0](https://github.com/mcpmux/mcp-mux/compare/v0.0.1...v0.1.0) (2026-02-16)

First public release of McpMux — the unified MCP gateway and manager for AI clients.

### Features

* Unified MCP gateway — configure servers once, connect every AI client through a single endpoint
* Encrypted credential storage via OS keychain (DPAPI, Keychain, Secret Service) with AES-256-GCM field-level encryption
* Spaces for organizing servers into workspaces with per-client access key authentication
* FeatureSet filtering — fine-grained control over tools, resources, and prompts per client
* OAuth 2.1 + PKCE with automatic token refresh for OAuth-enabled MCP servers
* Server discovery — browse and install from the community registry at mcpmux.com
* Streamable HTTP transport with SSE notifications
* Stdio transport with platform-specific process isolation
* Server connection logging with MCP protocol notifications and stderr capture
* Custom server configuration fields — environment variables, arguments, and headers
* Default values for server input definitions
* McpMux-branded OAuth authorization pages
* System tray with autostart on login
* Built-in auto-updater with signed releases
* Cross-platform installers — Windows (NSIS), macOS (DMG via Homebrew), Linux (APT + AppImage + .deb)
