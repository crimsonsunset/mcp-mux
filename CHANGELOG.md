# Changelog

## [0.6.0](https://github.com/crimsonsunset/mcp-mux/compare/v0.5.0...v0.6.0) (2026-08-21)


### Features

* [@mux](https://github.com/mux) UX + Windows updater fix + minimal-first optimization ([#171](https://github.com/crimsonsunset/mcp-mux/issues/171)) ([a215012](https://github.com/crimsonsunset/mcp-mux/commit/a215012ccd37388ffc6d802452e2fe03c9ce1ea5))
* add autostart and system tray functionality ([#38](https://github.com/crimsonsunset/mcp-mux/issues/38)) ([cc99fcf](https://github.com/crimsonsunset/mcp-mux/commit/cc99fcf412f24f48edba12b8f0359fa71b5247c6))
* Add custom server configuration fields (env vars, args, headers) ([#54](https://github.com/crimsonsunset/mcp-mux/issues/54)) ([37ce0f5](https://github.com/crimsonsunset/mcp-mux/commit/37ce0f575883680e2ee12354e3bfea48e7a9337e))
* add Homebrew tap support and ad-hoc macOS signing ([#79](https://github.com/crimsonsunset/mcp-mux/issues/79)) ([b07f1a3](https://github.com/crimsonsunset/mcp-mux/commit/b07f1a3a6a11fd1ae944368fa0909838ffb41292))
* add Linux APT repository and install infrastructure ([#85](https://github.com/crimsonsunset/mcp-mux/issues/85)) ([473eb1a](https://github.com/crimsonsunset/mcp-mux/commit/473eb1aeb25c8e01d2111b9e1a82e8fee1a4d4dd))
* add select, file_path, and directory_path input types ([#121](https://github.com/crimsonsunset/mcp-mux/issues/121)) ([942ee1a](https://github.com/crimsonsunset/mcp-mux/commit/942ee1ae88f60aa1454bc97cec3839bcacf74454))
* API-key inbound auth for headless/remote MCP clients (P1/3) ([#201](https://github.com/crimsonsunset/mcp-mux/issues/201)) ([4b5a9bc](https://github.com/crimsonsunset/mcp-mux/commit/4b5a9bcc73bc301ee7b9591079d7b1166049177a))
* apply McpMux branding to OAuth authorization pages ([#74](https://github.com/crimsonsunset/mcp-mux/issues/74)) ([c84e036](https://github.com/crimsonsunset/mcp-mux/commit/c84e036b13b276520b8433439954303dbe3dbaed))
* **auth:** Phase 1 — API-key inbound auth ([830c2ec](https://github.com/crimsonsunset/mcp-mux/commit/830c2ecf7fb64db7972b44dc4c0abfa1e9c79b1e))
* Capture and stream process stderr to server log manager ([#63](https://github.com/crimsonsunset/mcp-mux/issues/63)) ([96795b0](https://github.com/crimsonsunset/mcp-mux/commit/96795b0b54ecfaa9743bb9e6045bfc86ddadcc2f))
* capture MCP protocol logging notifications in server connection logs ([#76](https://github.com/crimsonsunset/mcp-mux/issues/76)) ([0587741](https://github.com/crimsonsunset/mcp-mux/commit/058774135fed2c0220a4900372f665e88eb3dff5))
* **clients:** let connections optionally associate with a machine ([df0f7b8](https://github.com/crimsonsunset/mcp-mux/commit/df0f7b8d205979861c21018611bcfc4c34d6fffd))
* **clients:** Phase 2 — unified Cursor/Generic tabs in Register client modal ([3fcf6ca](https://github.com/crimsonsunset/mcp-mux/commit/3fcf6ca5d83783c02a5184497ca406bd04668249))
* **clients:** Phase 3 — regenerate choice + remove redundant Cursor bridge card ([03aaae7](https://github.com/crimsonsunset/mcp-mux/commit/03aaae71f3a1b3cf683c30a22874d3b85718bc3f))
* **clone:** Phase 2 — DB-backed definition edit path for manual_entry clones ([88978bd](https://github.com/crimsonsunset/mcp-mux/commit/88978bdec04065b29620d355c6e2965d30f873f4))
* **clone:** Phase 4 — warn on clones with missing auth headers ([64bd5ef](https://github.com/crimsonsunset/mcp-mux/commit/64bd5ef6ab9d4a6f200ee419569e96b24c2f723c))
* **cursor-bridge:** Phase 2 — Desktop UI generator ([e48885c](https://github.com/crimsonsunset/mcp-mux/commit/e48885cd411e84e60f92f83b039e98bec0ce7144))
* **desktop,gateway:** Phase 5 — docs reconcile + remove dead collision_client_id wiring ([7345ff2](https://github.com/crimsonsunset/mcp-mux/commit/7345ff2715f5470ecc4f1caca40b208009a7231d))
* **desktop:** install a managed Cursor preToolUse workspace hook ([86c339e](https://github.com/crimsonsunset/mcp-mux/commit/86c339e9a06cd2d0beae62c74099787296a94f47))
* **desktop:** install the Cursor workspace hook from Connections ([1ca8878](https://github.com/crimsonsunset/mcp-mux/commit/1ca8878e2371dd9fc1ca294f9ea18e0f19f51dae))
* **desktop:** machine ID copy/link on viewer modal and Settings ([7c60234](https://github.com/crimsonsunset/mcp-mux/commit/7c60234dfb32fe61a35ea30401f09ce29f929f06))
* **desktop:** Phase 4 — projects cards deny CTA + unbound empty state ([64db1c7](https://github.com/crimsonsunset/mcp-mux/commit/64db1c705ac5da35766a64105425917eab9f3456))
* **desktop:** replace Zustand nav with wouter URL routing ([22e3237](https://github.com/crimsonsunset/mcp-mux/commit/22e3237462bf76002ad18aaa0771e3f597271e1e))
* **featureset:** protect Starter from deletion + clarify mapping popup ([#176](https://github.com/crimsonsunset/mcp-mux/issues/176)) ([163ee0b](https://github.com/crimsonsunset/mcp-mux/commit/163ee0b0ef0ac0166be0ecf9e2f8bad1612dfad3))
* file-based keychain fallback for headless Linux/WSL ([#103](https://github.com/crimsonsunset/mcp-mux/issues/103)) ([9b60e0b](https://github.com/crimsonsunset/mcp-mux/commit/9b60e0bbe47a2318e7352efd3ba8b1888f393f38))
* **gateway:** constrain workspace pins to the caller's open folder set ([efabe48](https://github.com/crimsonsunset/mcp-mux/commit/efabe48c21a28edb1235a1ffe9045bbfdbf837ca))
* **gateway:** default FeatureSet for unmapped roots + Mapped workspaces filter ([#175](https://github.com/crimsonsunset/mcp-mux/issues/175)) ([7fc50a0](https://github.com/crimsonsunset/mcp-mux/commit/7fc50a00923f74f752e5c279f60e232c1865c3e5))
* **gateway:** diagnose set-header assumptions; refresh routing docs ([0c1503b](https://github.com/crimsonsunset/mcp-mux/commit/0c1503b4fce2842d39ad869e59c514404b4f4396))
* **gateway:** enable remote MCP via public URL and tunnel consent ([3e488a2](https://github.com/crimsonsunset/mcp-mux/commit/3e488a2a619763349791f8a6aeefd3157b27a13a))
* **gateway:** exact per-call Cursor workspace routing ([ef03e78](https://github.com/crimsonsunset/mcp-mux/commit/ef03e786d433b08a2878b819a227c13bd09f5daf))
* **gateway:** honor _mcpmux_context on call_tool and meta tools ([5ed4db6](https://github.com/crimsonsunset/mcp-mux/commit/5ed4db625219b50899b6bf5d7a62ca6fb34fe65b))
* **gateway:** log workspace pin/session signals for Agents Window spike ([c0e3196](https://github.com/crimsonsunset/mcp-mux/commit/c0e3196ac3325f5b82c29720433667ceb8a15037))
* **gateway:** optional network access — bind 0.0.0.0 for LAN sharing ([#200](https://github.com/crimsonsunset/mcp-mux/issues/200)) ([9e481e7](https://github.com/crimsonsunset/mcp-mux/commit/9e481e71b5857d9f67b62a811694034bed3a4400))
* **gateway:** Phase 0 search_tools latency instrumentation ([102bb92](https://github.com/crimsonsunset/mcp-mux/commit/102bb9278eb9d8b76184eb4ad0913e12e395bdd0))
* **gateway:** Phase 1 — resolver deny by default (Unbound) ([472dca0](https://github.com/crimsonsunset/mcp-mux/commit/472dca0b7e672703f269b907d8ec76de4ba02cca))
* **gateway:** Phase 2 — invocation gate + self-bind escape hatch ([f2efc81](https://github.com/crimsonsunset/mcp-mux/commit/f2efc81a945c76c3066fa5df6be43d03b0fee40f))
* **gateway:** Phase 2 — repo-name matching for declared rootless roots ([38f1df1](https://github.com/crimsonsunset/mcp-mux/commit/38f1df1a41a5f5966ad63f6c526c915d5f278761))
* **gateway:** remember workspace pins per mcp-remote process ([5997a11](https://github.com/crimsonsunset/mcp-mux/commit/5997a11efa8d6dbf467c01fac8467c1b17995667))
* **gateway:** resolve bindings from X-Mcpmux-Machine-Id header ([65a33b7](https://github.com/crimsonsunset/mcp-mux/commit/65a33b700f6779897c43632569ca89210a6e6562))
* **gateway:** resolve FeatureSets from an explicit per-call workspace root ([f3e1103](https://github.com/crimsonsunset/mcp-mux/commit/f3e1103b7d03627387b6690ccef90e467b359aa1))
* **gateway:** web admin clone parity + fix dropped display-name/update-policy on save ([f024d9e](https://github.com/crimsonsunset/mcp-mux/commit/f024d9e3db7208b0e0523e70293c54bc991666dc))
* **gateway:** web admin version-probe parity and drop unused mcpmux-mcp ([a8a2f4a](https://github.com/crimsonsunset/mcp-mux/commit/a8a2f4af0e7b63b53bb310f093b77516be9fee2f))
* generalized client→Space/FeatureSet mappings + lock-confine (P2/3) ([#202](https://github.com/crimsonsunset/mcp-mux/issues/202)) ([2913ecb](https://github.com/crimsonsunset/mcp-mux/commit/2913ecb8098044734ea7ee105533d97834f6c956))
* implement Tauri updater functionality ([#36](https://github.com/crimsonsunset/mcp-mux/issues/36)) ([d355c68](https://github.com/crimsonsunset/mcp-mux/commit/d355c68a4b33901adb7f9be8c0765252f8c3577f))
* initial release of McpMux desktop app ([72181e2](https://github.com/crimsonsunset/mcp-mux/commit/72181e2b462f4f70eb586758e8bd029dcb3b7631))
* land fork on main — machine-scoped routing, web admin, remote MCP ([6f98ee7](https://github.com/crimsonsunset/mcp-mux/commit/6f98ee790fb7c9f3743ce73d267b49933c1d692b))
* Mapping/Clients rename + non-localhost consent note (P3/3) ([#203](https://github.com/crimsonsunset/mcp-mux/issues/203)) ([87df4a2](https://github.com/crimsonsunset/mcp-mux/commit/87df4a26fb13e159c09ea8d920c3ce01050c7488))
* **oauth:** add dismiss X button to all consent modal states ([be02730](https://github.com/crimsonsunset/mcp-mux/commit/be02730c208f3262773ca40adb5b1052aefcb9c4))
* **oauth:** machine naming on first connect + streamline browser bridge ([e1c7af8](https://github.com/crimsonsunset/mcp-mux/commit/e1c7af8b0687381a33a68a7dbe693b6f8b40b52a))
* per-workspace routing via X-Mcpmux-Workspace header + guided folder setup ([#182](https://github.com/crimsonsunset/mcp-mux/issues/182)) ([e2ec055](https://github.com/crimsonsunset/mcp-mux/commit/e2ec0558eada73407addc57902d9f13763cc8aec))
* **port:** Phase 1 — Foundation: shared UI library + backend facade ([46a92a1](https://github.com/crimsonsunset/mcp-mux/commit/46a92a1166915d8dd88dad5a619da64f3a1f73b1))
* **port:** Phase 2 — Storage layer: migration reconciliation + new repositories ([abf5ae6](https://github.com/crimsonsunset/mcp-mux/commit/abf5ae6bde060c87ecd85bc8cd0b64d584b983f7))
* **port:** Phase 3 — Web admin server stack ([e6fc66f](https://github.com/crimsonsunset/mcp-mux/commit/e6fc66fd55b450050b5c467f38d345390a3e8fc1))
* **port:** Phase 4 — macOS shell + Tauri features ([bcf9565](https://github.com/crimsonsunset/mcp-mux/commit/bcf95650d5758a4282d0847250335ff1979834f1))
* **port:** Phase 5 — Meta-tools enhancements ([124baa3](https://github.com/crimsonsunset/mcp-mux/commit/124baa36697f40cd40ce0ba6105e49ab5f0de32e))
* **port:** Phase 6 — Server features: cloning + update policy ([2e7256a](https://github.com/crimsonsunset/mcp-mux/commit/2e7256a6ddbacfb0a06cb163ccd7f10b7b170dd0))
* **port:** Phase 7 — Dashboard + workspace appearances ([bf1ee50](https://github.com/crimsonsunset/mcp-mux/commit/bf1ee50a8e10b99bb0f1a421ded64e659af232e6))
* **port:** Phase 8 — i18n rebase + landing ([7d9c6e6](https://github.com/crimsonsunset/mcp-mux/commit/7d9c6e6e52024410312c34680f93bf29f94285c6))
* post-action UX guidance, ConfirmDialog, and client auto-select ([#136](https://github.com/crimsonsunset/mcp-mux/issues/136)) ([44d934c](https://github.com/crimsonsunset/mcp-mux/commit/44d934c678c4d7a2eebc996928e2fb37c07d7a8e))
* pre-release update channel + automated pre-releases from main ([#159](https://github.com/crimsonsunset/mcp-mux/issues/159)) ([e9306c4](https://github.com/crimsonsunset/mcp-mux/commit/e9306c4a8ac1aee72be2530697a643b69fb130f6))
* redesign README, screenshots, and E2E capture ([#87](https://github.com/crimsonsunset/mcp-mux/issues/87)) ([84f15cb](https://github.com/crimsonsunset/mcp-mux/commit/84f15cb8b805912ca810ad00bff9f819802f4d78))
* **routing:** Phase 2 — id-type bindings + resolver Tier 2 ([9af05c0](https://github.com/crimsonsunset/mcp-mux/commit/9af05c0c1fc23ffebec6938285e5c69800f7751d))
* **routing:** Phase 3 — Space lock as narrowing filter ([3401053](https://github.com/crimsonsunset/mcp-mux/commit/3401053cb8a56efbb70481148dccd1802e839331))
* **servers:** allow editing custom server definitions ([cf69d3e](https://github.com/crimsonsunset/mcp-mux/commit/cf69d3e4e1d4b9134803e24c63cb0008ce586404))
* **servers:** Phase 1 — Manifest modal: fullscreen sizing + search fixes ([febafda](https://github.com/crimsonsunset/mcp-mux/commit/febafda2f52c5269ffad084d67d77343a1a1870b))
* **servers:** Phase 2 — Shared helpers + panel shell + JSON mode ([c0aa850](https://github.com/crimsonsunset/mcp-mux/commit/c0aa850667cd86a9bb6b8d7be4d23a663f9817b2))
* **servers:** Phase 3 — Panel Form mode ([c87c337](https://github.com/crimsonsunset/mcp-mux/commit/c87c3372567b89640d05b109cdae79866d4965f4))
* **servers:** Phase 4 — Wire entry points ([3c7fccf](https://github.com/crimsonsunset/mcp-mux/commit/3c7fccfe17e56e78e5e9f26efb3ab2c2c2dc5ae4))
* **servers:** write-only space config save with async file-watcher sync ([dfc0ebf](https://github.com/crimsonsunset/mcp-mux/commit/dfc0ebf48954f817c189c9bb45050dec15c0bc0a))
* **spaces:** per-space base directories scope workspace roots to a Space ([#179](https://github.com/crimsonsunset/mcp-mux/issues/179)) ([fb825cf](https://github.com/crimsonsunset/mcp-mux/commit/fb825cfe66c6f383ea1f52d290a5519a67bffd5f))
* Streamable HTTP transport with SSE notifications and E2E tests ([#61](https://github.com/crimsonsunset/mcp-mux/issues/61)) ([ca5b0ff](https://github.com/crimsonsunset/mcp-mux/commit/ca5b0ffab19aa395a75c5f10a18ab0e6efb1752a))
* support configurable public gateway base URL ([#192](https://github.com/crimsonsunset/mcp-mux/issues/192)) ([6f81378](https://github.com/crimsonsunset/mcp-mux/commit/6f81378384ed37cf254d7b8711199404a848ca3f))
* support default values for input definitions ([#70](https://github.com/crimsonsunset/mcp-mux/issues/70)) ([a1d9599](https://github.com/crimsonsunset/mcp-mux/commit/a1d9599601c212c1b7054fc4c5c76f065e0ea920))
* **sync:** Phase 1 — continue-on-error + adopt-on-conflict in user config sync ([93f06ca](https://github.com/crimsonsunset/mcp-mux/commit/93f06ca25c8efa1566fb402bf8db00afbf6b584f))
* **sync:** Phase 2 — surface adopted/error counts in sync-updated event ([150bb3e](https://github.com/crimsonsunset/mcp-mux/commit/150bb3effdfd3e1fe1e489a0378511df679692f8))
* **ui:** opencode global connect + client icons ([#184](https://github.com/crimsonsunset/mcp-mux/issues/184)) ([669e99f](https://github.com/crimsonsunset/mcp-mux/commit/669e99f2df3d85bbdd33427f0a96bd8e0048c7c3))
* **ui:** Phase 1 — SearchableSelect combobox component ([82093ae](https://github.com/crimsonsunset/mcp-mux/commit/82093ae2e271ebd59411f1ae2def503e9c48ba12))
* unify Settings machine identity with viewer status bar ([583c6cd](https://github.com/crimsonsunset/mcp-mux/commit/583c6cd0b51f0c96dd0ed73236abe50a4d04d392))
* update logo with bolder strokes and regenerate icons/screenshots ([#71](https://github.com/crimsonsunset/mcp-mux/issues/71)) ([68c292c](https://github.com/crimsonsunset/mcp-mux/commit/68c292c424671edf4a914484a7af570770fac71e))
* **web-admin:** machine picker in binding sheet, cross-surface auto-close, focus fix ([5d0e28c](https://github.com/crimsonsunset/mcp-mux/commit/5d0e28c6b9559c34101f727f3f02965dc8e7260f))
* **web-admin:** Phase 1 — Dev tooling ([101e382](https://github.com/crimsonsunset/mcp-mux/commit/101e38255d9d41d561b6bc5feba695525932e1fe))
* **web-admin:** Phase 2 — Config export HTTP routes ([58ce57a](https://github.com/crimsonsunset/mcp-mux/commit/58ce57a2f3da56ab87f79bed371634db3261ee6b))
* **web-admin:** Phase 3 — OAuth SSE fan-out + builtin channel ([f92df80](https://github.com/crimsonsunset/mcp-mux/commit/f92df80792b8fdb2cd69fe223f2fb3251420cd3a))
* **web-admin:** Phase 4 — Web file picker fallback ([4fd573c](https://github.com/crimsonsunset/mcp-mux/commit/4fd573c7b872af4441698797f542c31b3b5b222d))
* **web-admin:** Phase 5 — Dead code cleanup + test coverage ([781f672](https://github.com/crimsonsunset/mcp-mux/commit/781f672fbd463959eabcaeef21aa3a7faab6999a))
* wire viewer identity to machine catalog and require full profiles ([2028fc9](https://github.com/crimsonsunset/mcp-mux/commit/2028fc9c43f9dcacb51166182f8b60e63c8d6919))
* workspace-root routing + Tool Optimization ([@mux](https://github.com/mux)) self-management + UI live-sync ([#151](https://github.com/crimsonsunset/mcp-mux/issues/151)) ([d614853](https://github.com/crimsonsunset/mcp-mux/commit/d6148538b6f40644f9367d3c872bc1f4f2f7be63))
* **workspaces:** adopt-clone prefill and machine-aware binding panel ([4ab677b](https://github.com/crimsonsunset/mcp-mux/commit/4ab677bd4b178ec4c1fd8830e50a83d8fc84c3d9))
* **workspaces:** BOUND ELSEWHERE card state with always-clickable binding rows ([ef35d56](https://github.com/crimsonsunset/mcp-mux/commit/ef35d56b3d40c3b26b49ef4e866b260f90d97219))
* **workspaces:** bulk-clear unmapped folders + clearer approval opt-out ([#172](https://github.com/crimsonsunset/mcp-mux/issues/172)) ([09b561c](https://github.com/crimsonsunset/mcp-mux/commit/09b561c901170cac2c6546ef8680b313f292eee6))
* **workspaces:** compact routing table footer on project cards ([3311a0e](https://github.com/crimsonsunset/mcp-mux/commit/3311a0eb21e197bfd702a12346af19e377bd35d1))
* **workspaces:** emoji picker, inline machine editor, binding state fixes ([fb7ea58](https://github.com/crimsonsunset/mcp-mux/commit/fb7ea5877f0f4705405b638919c68df4230fab94))
* **workspaces:** forget single reported root per-card dismiss button ([3d138df](https://github.com/crimsonsunset/mcp-mux/commit/3d138df7bd63ee03925fe06ae5a3106a60573576))
* **workspaces:** machine-aware client binding and resolver lookup ([a0a5907](https://github.com/crimsonsunset/mcp-mux/commit/a0a5907d26a16b5146879c52af3a43f35bc957b1))
* **workspaces:** machine-scoped bindings and multi-device gateway routing ([e38a390](https://github.com/crimsonsunset/mcp-mux/commit/e38a3900ae373d8b355975c24666e2ed257271dd))
* **workspaces:** Phase 1 — bindingPanelStore Zustand slice ([8b1b1c6](https://github.com/crimsonsunset/mcp-mux/commit/8b1b1c63fd31a1768103deb0f8145cad8f13ac3e))
* **workspaces:** Phase 1 — Domain + storage ([72b63b4](https://github.com/crimsonsunset/mcp-mux/commit/72b63b4240b8d98631410a2ffb48521182a6b6e6))
* **workspaces:** Phase 1 — Multi-binding Entry model ([3fc4d50](https://github.com/crimsonsunset/mcp-mux/commit/3fc4d508bd3cca47f35af1fe1b5e7c18c87ab519))
* **workspaces:** Phase 1 — sibling detection + adopt step ([975f038](https://github.com/crimsonsunset/mcp-mux/commit/975f038ab8a79292b6e3ba2124c85f8cca25e782))
* **workspaces:** Phase 2 — extract BindingForm, machine multiselect ([15871f8](https://github.com/crimsonsunset/mcp-mux/commit/15871f8e579af249663e75bb018634872d02e7ff))
* **workspaces:** Phase 2 — Grouped card UI ([d62d79d](https://github.com/crimsonsunset/mcp-mux/commit/d62d79dc8ee0f00a670f3006a49f3314db9726d7))
* **workspaces:** Phase 2 — LIVE badge fix for cross-machine paths ([45b539c](https://github.com/crimsonsunset/mcp-mux/commit/45b539caa081cd5f218d13085fdb193df3b7b89a))
* **workspaces:** Phase 2 — Settings layer + Tauri/admin commands ([0db6805](https://github.com/crimsonsunset/mcp-mux/commit/0db68051206e97e97c5bbd988be32419a4f155f5))
* **workspaces:** Phase 3 — Binding CRUD with machine dimension ([3653f4b](https://github.com/crimsonsunset/mcp-mux/commit/3653f4be8415ba6b108231f8ef19aef1802a9178))
* **workspaces:** Phase 3 — Inspector routing for grouped cards ([2640b0f](https://github.com/crimsonsunset/mcp-mux/commit/2640b0fa61f232104375ec866deb1f2c6a16f6a6))
* **workspaces:** Phase 3 — WorkspaceBindingPanel unified overlay ([c638783](https://github.com/crimsonsunset/mcp-mux/commit/c6387832ea2ba2016e553adb50e84be077927389))
* **workspaces:** Phase 4 — Resolver machine-aware lookup ([83dac51](https://github.com/crimsonsunset/mcp-mux/commit/83dac514c3771689e66f6817c89631232aa32a1d))
* **workspaces:** Phase 4 — wire WorkspaceBindingPanel into App.tsx ([07310d9](https://github.com/crimsonsunset/mcp-mux/commit/07310d96c0c8e86c09c7bc7bcdf4c2587bc64ab5))
* **workspaces:** Phase 5 — Projects UI machine filter and identity ([4a9f31f](https://github.com/crimsonsunset/mcp-mux/commit/4a9f31fa689c22b37eb7abd3e1b854a8d582bedf))
* **workspaces:** Phase 5 — WorkspacesPage drives bindingPanelStore ([75e55ab](https://github.com/crimsonsunset/mcp-mux/commit/75e55ab41ab88ee755c4e36201675cb6692b4c34))
* **workspaces:** Phase 6 — delete WorkspaceBindingSheet, cleanup, validate ([b688259](https://github.com/crimsonsunset/mcp-mux/commit/b688259898d85af3840dcc4dc6519822f29fae62))
* **workspaces:** setting to disable the new-folder mapping prompt ([#177](https://github.com/crimsonsunset/mcp-mux/issues/177)) ([d5df002](https://github.com/crimsonsunset/mcp-mux/commit/d5df002df03e9b80a0bd780ce9c52fd9e942d02e))
* **workspaces:** two-zone EntryCard footer with machine emoji chips ([0d884d0](https://github.com/crimsonsunset/mcp-mux/commit/0d884d006019a3a30c9c807cf413cacb1ca7cf4b))
* **workspaces:** workspace binding label/icon port (migration 032) ([18a7976](https://github.com/crimsonsunset/mcp-mux/commit/18a79761776fc89f30b5677f70f0be0c0859185f))
* **workspaces:** workspace binding UI refactor + deny-by-default planning ([4750935](https://github.com/crimsonsunset/mcp-mux/commit/4750935d7568c90de91a6fa188307e06190aebb1))


### Bug Fixes

* add one-click IDE install for VS Code and Cursor ([#119](https://github.com/crimsonsunset/mcp-mux/issues/119)) ([5b280fb](https://github.com/crimsonsunset/mcp-mux/commit/5b280fbfdcd04165827b7662ba6896cea96deb83))
* add projectPath to tauri-action for monorepo support ([0299a23](https://github.com/crimsonsunset/mcp-mux/commit/0299a23c5f995b4bae670ef709134967a19c6ee3))
* add Windsurf, JetBrains, and Android Studio to quick-connect grid ([#139](https://github.com/crimsonsunset/mcp-mux/issues/139)) ([fb58d9c](https://github.com/crimsonsunset/mcp-mux/commit/fb58d9ce6c46ec1a55356a9fecb35f34ae2b29f6))
* allow process restart after update and detect Homebrew version mismatch ([#134](https://github.com/crimsonsunset/mcp-mux/issues/134)) ([ecdbaca](https://github.com/crimsonsunset/mcp-mux/commit/ecdbacafaff573f497ce6db8614fa39993a28a32))
* avoid synthetic connecting state for enabled servers ([#196](https://github.com/crimsonsunset/mcp-mux/issues/196)) ([80045c6](https://github.com/crimsonsunset/mcp-mux/commit/80045c61e9ea2e92a035ab59f6d0adbad33175e0))
* Claude client icon resolving ([#68](https://github.com/crimsonsunset/mcp-mux/issues/68)) ([c54128e](https://github.com/crimsonsunset/mcp-mux/commit/c54128e0fbff96bd110de4e4dea45580dfad224c))
* **clients:** use primary color for active register-client tab ([9c478d2](https://github.com/crimsonsunset/mcp-mux/commit/9c478d28b8fc895ceda8c20eb303f0984aafeb8f))
* **clone:** Phase 1 — clone-time source rewrite and auth header seeding ([051b854](https://github.com/crimsonsunset/mcp-mux/commit/051b85481f90ccf020fe6a8eef11336163a8d0e6))
* debounce analytics search tracking to capture final query ([#132](https://github.com/crimsonsunset/mcp-mux/issues/132)) ([0f17ddb](https://github.com/crimsonsunset/mcp-mux/commit/0f17ddb768b5d309a3a73cc6df492f656e205f69))
* **deps:** resolve 4 transitive security advisories failing Dependabot ([#174](https://github.com/crimsonsunset/mcp-mux/issues/174)) ([eb32289](https://github.com/crimsonsunset/mcp-mux/commit/eb32289ca7f309c52223f17cf3a1e1c0f0a61d7c))
* **desktop:** restore emoji picker and Monaco modals in Tauri prod ([26318b1](https://github.com/crimsonsunset/mcp-mux/commit/26318b16b8f949c1205c45a5d2a75fcb223b7bff))
* detect OAuth requirement from unexpected content-type responses ([#128](https://github.com/crimsonsunset/mcp-mux/issues/128)) ([d894d17](https://github.com/crimsonsunset/mcp-mux/commit/d894d17c7c4c5841b7eb39dc1d7068dbcb447656))
* **dev:** proxy /api in dev:admin for browser tab parity ([0706736](https://github.com/crimsonsunset/mcp-mux/commit/070673649a91bd209b87f21e905b395272549631))
* don't pass APPLE_CERTIFICATE to tauri-action ([1943134](https://github.com/crimsonsunset/mcp-mux/commit/19431347c63eba3ed00b408d3e6c044bd3ac8a9c))
* e2e flaky fix ([#75](https://github.com/crimsonsunset/mcp-mux/issues/75)) ([d8e28f8](https://github.com/crimsonsunset/mcp-mux/commit/d8e28f8fd7a6ab50d3a12d766b970d482ae2fbe2))
* enable createUpdaterArtifacts for updater signatures ([c620e56](https://github.com/crimsonsunset/mcp-mux/commit/c620e56f9cae7cea4b7682ff49be37da3d1f670e))
* filter invalid DCR redirect URIs ([#193](https://github.com/crimsonsunset/mcp-mux/issues/193)) ([187b57c](https://github.com/crimsonsunset/mcp-mux/commit/187b57c31f68519dd536ee8e576cbae0b9a2cf08))
* Fix refresh token issue ([#69](https://github.com/crimsonsunset/mcp-mux/issues/69)) ([0eba047](https://github.com/crimsonsunset/mcp-mux/commit/0eba047922d3313121b5dd89e62f8b6aae9fe1db))
* **gateway:** annotate rng.gen as u8 for Windows inference ([2b4765b](https://github.com/crimsonsunset/mcp-mux/commit/2b4765b730842e87a10e27dc0dafe2ae5cba6bd7))
* **gateway:** close aug14 gateway ops bugs ([f4cf81c](https://github.com/crimsonsunset/mcp-mux/commit/f4cf81cef7f181b2c179eeca27b3a7c622cd18ea))
* **gateway:** close PR 11 review gaps on pins, reconnect, and probe ([83c3e16](https://github.com/crimsonsunset/mcp-mux/commit/83c3e161ebf0f671e9c53a515a1c1f8ba5ed71d8))
* **gateway:** close review gaps on cache, signals, and detached stop ([138e3fa](https://github.com/crimsonsunset/mcp-mux/commit/138e3fa661b34ddc7e737551e48f7ebc237dbf87))
* **gateway:** complete remote MCP OAuth path and CF service-token dev UX ([3bf03f5](https://github.com/crimsonsunset/mcp-mux/commit/3bf03f5788e42de7b8626de4bf5062ee74346d7f))
* **gateway:** disambiguate multi-root sessions via filesystem existence ([7ac5dc1](https://github.com/crimsonsunset/mcp-mux/commit/7ac5dc1d3bc7821012141b82d43dfa80a8e4efe7))
* **gateway:** drop resolution cache on disconnect; unstick TS tests ([934b0f2](https://github.com/crimsonsunset/mcp-mux/commit/934b0f22cb95542df07bdd1bc40154e027dc458d))
* **gateway:** gate ambiguous multi-root sessions to PendingRoots ([ee12700](https://github.com/crimsonsunset/mcp-mux/commit/ee127002f855870ebe2ee0996b697f6e4f019789))
* **gateway:** keep one-folder pins and redact probe env ([5e2e87c](https://github.com/crimsonsunset/mcp-mux/commit/5e2e87ccbe0611179db93c896b2f751a935d3bfc))
* **gateway:** normalize transport-closed error matching ([2da2f50](https://github.com/crimsonsunset/mcp-mux/commit/2da2f50c363ff8524541971c7197a29a0006c00f))
* **gateway:** Phase 1 — declare-root-before-grant gate for rootless clients ([dff476f](https://github.com/crimsonsunset/mcp-mux/commit/dff476f6c6da8834490bd8276cb40bec7096168b))
* **gateway:** Phase 1 — dev delta audit + cherry-pick ([af9a26f](https://github.com/crimsonsunset/mcp-mux/commit/af9a26f05132d69601af9aa72d77462139dc7697))
* **gateway:** Phase 1 — Surfacing list_* paths ([3d00dab](https://github.com/crimsonsunset/mcp-mux/commit/3d00dab111b4ecfe718c834a27569c2605429b5c))
* **gateway:** Phase 2 — call_* hard-cut guards ([12a2acd](https://github.com/crimsonsunset/mcp-mux/commit/12a2acd647eb7bba815ac2e98783817b3e9bff5a))
* **gateway:** Phase 3 — evict pool instance on ServerConfigUpdated ([5efd85d](https://github.com/crimsonsunset/mcp-mux/commit/5efd85db4c999518797226d7376f6b7d942af60c))
* **gateway:** Phase 4 — gateway fixes + consent polish ([3eb678c](https://github.com/crimsonsunset/mcp-mux/commit/3eb678c4d7e1be335d4e946fa74dbadd0993c16f))
* **gateway:** preserve structured tool results ([#206](https://github.com/crimsonsunset/mcp-mux/issues/206)) ([6bd8220](https://github.com/crimsonsunset/mcp-mux/commit/6bd822063dc11ff00bf90af18e6d426e37abb84e))
* **gateway:** quiet log noise, cache resolve_feature_sets, warn on empty workspace header ([5de93d5](https://github.com/crimsonsunset/mcp-mux/commit/5de93d54520b6e885c2399ae872eacdf8788ed15))
* **gateway:** reconnect after config save and hold workspace pin across initialize ([dcc2977](https://github.com/crimsonsunset/mcp-mux/commit/dcc2977f601b8fbd3ef403fa70660a68081c0780))
* **gateway:** refuse to promote unproven meta-tool pins to a window ([7d8436f](https://github.com/crimsonsunset/mcp-mux/commit/7d8436f5fd0effa0ad3bf9cc510a7f7ce9dd6490))
* **gateway:** reject multi-root bind with recoverable root list ([d88ee03](https://github.com/crimsonsunset/mcp-mux/commit/d88ee03225d557ce26098fd2b814191d37a3bc82))
* **gateway:** restore advertised tools filter + add surfacing regression plan ([9306bcf](https://github.com/crimsonsunset/mcp-mux/commit/9306bcf2cc638d17bd910d62c1d6ed6c92eed839))
* **gateway:** restore disabled auth on auto-start ([#205](https://github.com/crimsonsunset/mcp-mux/issues/205)) ([460da1f](https://github.com/crimsonsunset/mcp-mux/commit/460da1f2b81cb9aad403392d32d3caa2856b8a92))
* **gateway:** restore web admin OAuth consent bridge ([4aabdc6](https://github.com/crimsonsunset/mcp-mux/commit/4aabdc656f6e949754df59b79038dbe3f8e214b7))
* **gateway:** retry call_tool after a dead backend connection ([c09e569](https://github.com/crimsonsunset/mcp-mux/commit/c09e5699437aaf94761cc96d71c815f630adec9d))
* **gateway:** ride out port release race on restart; add machine assignment to API key clients ([259a3c0](https://github.com/crimsonsunset/mcp-mux/commit/259a3c054a842f25729521a3a7a8658d1b52e98e))
* **gateway:** ride out self-update port race + clearer update restart UX ([#173](https://github.com/crimsonsunset/mcp-mux/issues/173)) ([6868992](https://github.com/crimsonsunset/mcp-mux/commit/6868992faeb77c8fe32ad7c996b2d196ca586002))
* **gateway:** skip pending workspace pins from another window ([13bede1](https://github.com/crimsonsunset/mcp-mux/commit/13bede151a8e8c6a9876a924a3c84d207b673956))
* **gateway:** supersede prior same-root session on reconnect ([53d28df](https://github.com/crimsonsunset/mcp-mux/commit/53d28dff988e644475eeb1205af389e9181edf11))
* **gateway:** suppress dead_code + manual_non_exhaustive clippy lints ([1e7c51f](https://github.com/crimsonsunset/mcp-mux/commit/1e7c51f0abd8e9fb037a7b0743dd50af617322c9))
* **gateway:** treat rmcp "transport closed" as reconnectable ([54d0de2](https://github.com/crimsonsunset/mcp-mux/commit/54d0de25c5ffead2d663ac9db7b76aa2fb28c50e))
* **gateway:** truly no-auth when inbound auth is disabled (no OAuth advertising) ([#187](https://github.com/crimsonsunset/mcp-mux/issues/187)) ([3e617fd](https://github.com/crimsonsunset/mcp-mux/commit/3e617fd876360f097fbffa96e21ce1fb90013fea))
* **gateway:** wire admin SSE consent on gateway auto-start ([eb62a32](https://github.com/crimsonsunset/mcp-mux/commit/eb62a32a7574295aed65f57c5e92fc4552a05178))
* **gateway:** write machine-scoped bindings from meta-tools bind ([947b40c](https://github.com/crimsonsunset/mcp-mux/commit/947b40c556ac756905eb265ed4aeb0f37da4ca1b))
* gracefully handle invalid Apple certificate in release builds ([bb4221f](https://github.com/crimsonsunset/mcp-mux/commit/bb4221f9e4a47ff7fad041b13e432a2ed55e1f96))
* make feature discovery capability-aware and bounded ([#194](https://github.com/crimsonsunset/mcp-mux/issues/194)) ([c2a1569](https://github.com/crimsonsunset/mcp-mux/commit/c2a156974004e113b4368dada41411e61c8205f1))
* **oauth:** DCR skip-invalid redirect URIs + drop duplicate RFC 8707 resource param ([#158](https://github.com/crimsonsunset/mcp-mux/issues/158)) ([661f162](https://github.com/crimsonsunset/mcp-mux/commit/661f1620105803acfe07087e997a1d4d00aa77d5))
* **oauth:** de-duplicate deep-link handling + quiet status-poll log ([#189](https://github.com/crimsonsunset/mcp-mux/issues/189)) ([9dd7b58](https://github.com/crimsonsunset/mcp-mux/commit/9dd7b58a23cef1f7f6a5ce53cab20750270c1a26))
* **oauth:** remove credential caching to enable automatic token refresh ([#33](https://github.com/crimsonsunset/mcp-mux/issues/33)) ([f398cfa](https://github.com/crimsonsunset/mcp-mux/commit/f398cfad7f1f92956f528b5e4640049de77b5ac3))
* **ops:** close aug14 gateway bugs and detach agent-owned dev:admin ([7089c7c](https://github.com/crimsonsunset/mcp-mux/commit/7089c7c43435d30d759016f71b9185da8f07d5c4))
* **port:** Phase 3 — Data integrity regressions ([a1636e1](https://github.com/crimsonsunset/mcp-mux/commit/a1636e1348e7639c7c24c5195510553c8886fa09))
* **rebase:** post-rebase cleanup for settings, schema test, and resolver tests ([f40fdbb](https://github.com/crimsonsunset/mcp-mux/commit/f40fdbb91fcb9e41ac298f1734aa9bbc6865f5b1))
* regenerate ICO with proper sizes & increase connection timeout ([#123](https://github.com/crimsonsunset/mcp-mux/issues/123)) ([2d88b25](https://github.com/crimsonsunset/mcp-mux/commit/2d88b259e9ca1bbc1ac57405854d732d8437cce3))
* remove notarization env vars from tauri-action ([31ed4ed](https://github.com/crimsonsunset/mcp-mux/commit/31ed4ed7a1aa8b14bb6f403939fa62652a1579d3))
* render server icon URLs as images instead of raw text ([#57](https://github.com/crimsonsunset/mcp-mux/issues/57)) ([5a94708](https://github.com/crimsonsunset/mcp-mux/commit/5a94708dcddd47c26183bd18c6abd348c91f976c))
* replace deprecated macos-13 runner with macos-latest ([92ad770](https://github.com/crimsonsunset/mcp-mux/commit/92ad7702d2cada69df46c3a221bc2101caf17a20))
* resolve npx/node PATH on macOS GUI apps ([#113](https://github.com/crimsonsunset/mcp-mux/issues/113)) ([98c013d](https://github.com/crimsonsunset/mcp-mux/commit/98c013d4e6955e678949df6068c038e1b8cf00fc))
* **resolver:** match client+machine scoped bindings in find_exact_for_machine ([007de1f](https://github.com/crimsonsunset/mcp-mux/commit/007de1f639605e7856461462d9cc016a96ecf5d1))
* **resolver:** Phase 1 — Root-cause and fix Tier 1 binding mismatch ([4001253](https://github.com/crimsonsunset/mcp-mux/commit/4001253df8cae93447445f9ed673b207f6657741))
* restore titlebar drag region without breaking controls ([#197](https://github.com/crimsonsunset/mcp-mux/issues/197)) ([47c787c](https://github.com/crimsonsunset/mcp-mux/commit/47c787cf95b1c36e99673596170666252c59f114))
* **servers:** always show clone/add-account option, resolve to original source ([f786200](https://github.com/crimsonsunset/mcp-mux/commit/f786200688292e0825fcb57ecf3fe31e850f47f8))
* **servers:** pin config-modal footer ([#163](https://github.com/crimsonsunset/mcp-mux/issues/163)) + silent Windows updates ([#165](https://github.com/crimsonsunset/mcp-mux/issues/165)) ([0ddbdb5](https://github.com/crimsonsunset/mcp-mux/commit/0ddbdb59c7229f8bd9dd0a4875216af5ab8977af))
* **servers:** polish custom server panel header layout ([f0355de](https://github.com/crimsonsunset/mcp-mux/commit/f0355de65e22e29d64e839c3da4a6af76c9be1d0))
* **servers:** stop auto-inserting duplicate draft on custom server add ([77eae69](https://github.com/crimsonsunset/mcp-mux/commit/77eae695c8863fc9635d3953cb520c5099cf5f7c))
* **settings:** show viewer-only machine identity for remote web admin ([4ae408a](https://github.com/crimsonsunset/mcp-mux/commit/4ae408a5e446938585ab39d39e74995e5b5ea365))
* skip Apple certificate in tauri-action when import fails ([968d4b9](https://github.com/crimsonsunset/mcp-mux/commit/968d4b90b09bffc9a2432a32f2318ffde3facb88))
* **spaces:** clearer base-directories UX ([#180](https://github.com/crimsonsunset/mcp-mux/issues/180)) ([4a69908](https://github.com/crimsonsunset/mcp-mux/commit/4a699085087b775439180ace2467cda708b26fc3))
* stdio enable error UI state ([#104](https://github.com/crimsonsunset/mcp-mux/issues/104)) ([b4598e6](https://github.com/crimsonsunset/mcp-mux/commit/b4598e60e12d3389717fc2252bac8eb29e96f9c9))
* **storage:** drop a deleted FeatureSet from workspace bindings ([#186](https://github.com/crimsonsunset/mcp-mux/issues/186)) ([5598451](https://github.com/crimsonsunset/mcp-mux/commit/559845193aae3bc5bffff1e41758ffce1c083625))
* **storage:** Phase 3 — Backfill stale approved OAuth client flags ([b1680a5](https://github.com/crimsonsunset/mcp-mux/commit/b1680a50555e0b3e1ae513ad1ced0c9c26370058))
* **storage:** purge orphaned feature_set_members after the refactor (migration 017) ([#167](https://github.com/crimsonsunset/mcp-mux/issues/167)) ([b90b05c](https://github.com/crimsonsunset/mcp-mux/commit/b90b05c038d9d1bc8fae395e83e0e8db713d3e3f))
* **storage:** reconcile fork-era DB migration numbering on upgrade ([bed5e90](https://github.com/crimsonsunset/mcp-mux/commit/bed5e90f3e5bbd1cc847009ef66578aa5396127e))
* **storage:** use as_chunks so clippy 1.98 is happy ([960c343](https://github.com/crimsonsunset/mcp-mux/commit/960c34374247b553cc78d48643c3c9d5ae116d1e))
* suppress console window for stdio MCP servers on Windows ([#59](https://github.com/crimsonsunset/mcp-mux/issues/59)) ([98f862c](https://github.com/crimsonsunset/mcp-mux/commit/98f862cae83f24c4397fe8e6204215c68b0baf92))
* sync custom server config saves immediately ([#195](https://github.com/crimsonsunset/mcp-mux/issues/195)) ([536a4ce](https://github.com/crimsonsunset/mcp-mux/commit/536a4ceb7e4ad32549f870145557d00fe790cb2e))
* taskbar icon visibility ([#83](https://github.com/crimsonsunset/mcp-mux/issues/83)) ([400c4bc](https://github.com/crimsonsunset/mcp-mux/commit/400c4bcce315bc7354dacb40b1cfa95a51e0edd3))
* **tests:** use renderWithI18n in ConnectIDEs opencode test ([425b44a](https://github.com/crimsonsunset/mcp-mux/commit/425b44aede602cff1297d8cc1c601db5059effea))
* **ui:** scroll-to + flash the target Settings section on every redirect ([#190](https://github.com/crimsonsunset/mcp-mux/issues/190)) ([e032c9b](https://github.com/crimsonsunset/mcp-mux/commit/e032c9bc2e5a99ea64df4a4af215c5a29153af52))
* **ui:** show official opencode logo in the Apps tab ([#185](https://github.com/crimsonsunset/mcp-mux/issues/185)) ([608a841](https://github.com/crimsonsunset/mcp-mux/commit/608a841b1b0d058e055ff2ff0ba6cc37ed921a33))
* Update screenshots and Logo ([#56](https://github.com/crimsonsunset/mcp-mux/issues/56)) ([e6fb736](https://github.com/crimsonsunset/mcp-mux/commit/e6fb736ca7a79c13f227cdb470e735df447ea7cd))
* ux improvements and fixes ([#42](https://github.com/crimsonsunset/mcp-mux/issues/42)) ([fa52576](https://github.com/crimsonsunset/mcp-mux/commit/fa52576fc79102af992f71ef059f8f7eb937a23d))
* version display & update check ([#117](https://github.com/crimsonsunset/mcp-mux/issues/117)) ([b40c59b](https://github.com/crimsonsunset/mcp-mux/commit/b40c59bfb7b9ec19be8848abe04e38ba6fed1422))
* **web-admin:** complete admin HTTP routes and browser event transport ([864e7b5](https://github.com/crimsonsunset/mcp-mux/commit/864e7b53a14ba3622ea1e78ba715cf68a4c96b38))
* **web-admin:** consolidate OAuth SSE onto shared admin hub ([536c62a](https://github.com/crimsonsunset/mcp-mux/commit/536c62aa271c6e76d88abb1033261ea6892eb18e))
* **web-admin:** Phase 2 — lib/api invoke to apiCall migration ([01cae78](https://github.com/crimsonsunset/mcp-mux/commit/01cae786f063c6c6807a12ace914c1a11661e02c))
* wire up HTTP definition headers orthogonally from auth ([#125](https://github.com/crimsonsunset/mcp-mux/issues/125)) ([04380e0](https://github.com/crimsonsunset/mcp-mux/commit/04380e0979ab428351185d381001d209e6a4993b))
* **workspace-binding:** expand ~ home shorthand in reported/pinned roots ([b66cc54](https://github.com/crimsonsunset/mcp-mux/commit/b66cc547356f8345893f60305f1dff00ba6f576c))
* **workspace-binding:** Phase 2 — Persist WorkspaceNeedsBinding dismissals ([8d752fb](https://github.com/crimsonsunset/mcp-mux/commit/8d752fb84051b0e3aa76e27e4deb226a1b6913d3))
* **workspaces:** align machine API args with Tauri command signatures ([e1d88e0](https://github.com/crimsonsunset/mcp-mux/commit/e1d88e0134a22ac98745c4433570323767dca2ed))
* **workspaces:** auto-prefill adopt from same-path cross-machine bindings ([d622f5c](https://github.com/crimsonsunset/mcp-mux/commit/d622f5cc1c6a5cbce891f3add10f924be226c47f))
* **workspaces:** keep edit panel open; header machine quick-switch ([3da03b9](https://github.com/crimsonsunset/mcp-mux/commit/3da03b94fb783d879a6ef3ec159152caf8e33441))
* **workspaces:** restore binding panel regressions and UX tweaks ([1cd6c86](https://github.com/crimsonsunset/mcp-mux/commit/1cd6c86bde2fcb8dc485d3081056a1ffba88e927))
* **workspaces:** wrap feature set names in routing table rows ([a68073c](https://github.com/crimsonsunset/mcp-mux/commit/a68073cf913bd54bade30ad06be65c85500cb071))


### Performance

* **gateway:** Phase 1 search_tools — drop readiness resolve, Arc cache ([cf36934](https://github.com/crimsonsunset/mcp-mux/commit/cf36934185e4170fd7b7b09990dff7dd48b6fd03))


### Refactoring

* **frontend:** fix react-refresh lint warnings, add rollout plan doc ([f5cf27a](https://github.com/crimsonsunset/mcp-mux/commit/f5cf27a51dcc9644b789531d884df993ba7ff1b6))
* remove Password and Textarea from InputType enum ([#122](https://github.com/crimsonsunset/mcp-mux/issues/122)) ([bd06386](https://github.com/crimsonsunset/mcp-mux/commit/bd06386e04020da381135761a631ab38543ae414))
* **ui:** apply fork nav renames across sidebar and dashboard ([2f4384f](https://github.com/crimsonsunset/mcp-mux/commit/2f4384f6496b32f4a963d29db60d3171a6f8e0da))
* **ui:** remove Home and unify on Dashboard landing ([f55ddef](https://github.com/crimsonsunset/mcp-mux/commit/f55ddef67f9e17afcd3bf5d39f6d83e88debf9c3))
* **workspaces:** Phase 1 — Lift form state to panel ([138dba7](https://github.com/crimsonsunset/mcp-mux/commit/138dba7c046564754b22b066f830b5243b7b7b24))
* **workspaces:** Phase 2 — Panel header identity + machine badge ([fadda96](https://github.com/crimsonsunset/mcp-mux/commit/fadda96e7c204d442862311022b183390960566e))
* **workspaces:** Phase 3 — Routing + Scope sections, badge wiring ([c98fda7](https://github.com/crimsonsunset/mcp-mux/commit/c98fda7b76da3bebd87acc422c3335b9ac6811b1))


### Documentation

* add comprehensive light-theme screenshots for all features ([#47](https://github.com/crimsonsunset/mcp-mux/issues/47)) ([cefa644](https://github.com/crimsonsunset/mcp-mux/commit/cefa644daad6c23640db6bc767eb1dd0e43199f0))
* add Discord community link to README ([#149](https://github.com/crimsonsunset/mcp-mux/issues/149)) ([c32f78f](https://github.com/crimsonsunset/mcp-mux/commit/c32f78f7143177589ea96b4e33170f49cc343b30))
* add mcpmux.com links and download references to README ([#64](https://github.com/crimsonsunset/mcp-mux/issues/64)) ([04ab100](https://github.com/crimsonsunset/mcp-mux/commit/04ab1006d68860ff6f347ef6cd68dd3d678c3352))
* add remote access guide for tunnel and CF Access setup ([ec03463](https://github.com/crimsonsunset/mcp-mux/commit/ec034637ddcdaf57417c16a811ad9ceef54f640e))
* add upstream client mapping reconciliation plan ([30b2a17](https://github.com/crimsonsunset/mcp-mux/commit/30b2a173fe2c335212aa33ac65a385b8c63f4c2b))
* add user guide with screenshots ([#130](https://github.com/crimsonsunset/mcp-mux/issues/130)) ([a97a133](https://github.com/crimsonsunset/mcp-mux/commit/a97a1333520fc1ac54f061344970cf493807ca87))
* add user guide with screenshots ([#131](https://github.com/crimsonsunset/mcp-mux/issues/131)) ([ee28e8b](https://github.com/crimsonsunset/mcp-mux/commit/ee28e8be432d2b1532f3f98067ba9004c4a18374))
* add workspace machine binding planning docs ([5101128](https://github.com/crimsonsunset/mcp-mux/commit/5101128b8ecb4190d4590a455ac9f0ad2618e154))
* add Workspaces and Tool Optimization guides ([#164](https://github.com/crimsonsunset/mcp-mux/issues/164)) ([25a6fbb](https://github.com/crimsonsunset/mcp-mux/commit/25a6fbb94d3efcbd0cd5a714ac452c5e61608250))
* complete the getting-started flow + workspace-driven routing ([#166](https://github.com/crimsonsunset/mcp-mux/issues/166)) ([92f8ac2](https://github.com/crimsonsunset/mcp-mux/commit/92f8ac2f053e27b4a1aec222eea7ff3f9986559c))
* comprehensive README rewrite with features, security, and archi… ([#44](https://github.com/crimsonsunset/mcp-mux/issues/44)) ([243d3a3](https://github.com/crimsonsunset/mcp-mux/commit/243d3a369f34cfb02c5e82085e90c68e5bed963d))
* **cursor-bridge:** Phase 3 — Docs consolidation ([bda7e04](https://github.com/crimsonsunset/mcp-mux/commit/bda7e047332d79ec90f36b4d9b832f1b0067356c))
* improve README first impression with problem/fix diagrams ([#109](https://github.com/crimsonsunset/mcp-mux/issues/109)) ([b15482b](https://github.com/crimsonsunset/mcp-mux/commit/b15482b32a016e3ca92753f26212f5827f744903))
* per-device machine header and identity copy paths ([3828da4](https://github.com/crimsonsunset/mcp-mux/commit/3828da4d9fcb2ae5db284a3beb8d72c103cffedc))
* plan harness adapters around three workspace signals ([f1d967f](https://github.com/crimsonsunset/mcp-mux/commit/f1d967fd9d1accbf9833a8a3017db8f582e61dde))
* **planning:** add backend-connection-resilience plan, mark aug14-gateway-ops-bugs stale ([1367048](https://github.com/crimsonsunset/mcp-mux/commit/1367048026f3187c6e3a3871cc90a11de511f0ab))
* **planning:** add clone auth header and custom server panel plans ([399dd4f](https://github.com/crimsonsunset/mcp-mux/commit/399dd4f38cc94e89536ccea92a26f73d917d11ac))
* **planning:** add dev-rebased post-port completion plan ([1c65d6b](https://github.com/crimsonsunset/mcp-mux/commit/1c65d6bceb121cbabd8ac68c5ae6f08b41d86122))
* **planning:** add MCP 2026-07-28 spec-impact analysis, close upstream reconciliation ([0d784c8](https://github.com/crimsonsunset/mcp-mux/commit/0d784c80070cbde4235b42c7c409c0c21b442c45))
* **planning:** add pool-invalidation manual test playbook and Aug 20 results ([508930e](https://github.com/crimsonsunset/mcp-mux/commit/508930e1f5a41fabc6dfd6b3772b17d91f57a442))
* **planning:** add unified Register client modal plan ([aa76566](https://github.com/crimsonsunset/mcp-mux/commit/aa765667fa229c63eb720c78f5648e15e85dad5c))
* **planning:** add web admin completion plan ([9bb6e7a](https://github.com/crimsonsunset/mcp-mux/commit/9bb6e7ad660cdea0d1bd18db8bcde9dec1b67eea))
* **planning:** close search-tools-perf — warm path done, widen parked ([58f3852](https://github.com/crimsonsunset/mcp-mux/commit/58f385221b75164070eae425b38b5c01f8a2b27e))
* **planning:** mark web admin clone parity as shipped ([ae445a1](https://github.com/crimsonsunset/mcp-mux/commit/ae445a1e896067a79fca7aa12070d621cd301416))
* **planning:** mark workspace binding popup loop fix implemented ([51209c5](https://github.com/crimsonsunset/mcp-mux/commit/51209c5fc7450205e2ebe065a46f25ded6f9cf10))
* **planning:** Phase 6 — verification pass + reconcile surfacing plan ([4ddc339](https://github.com/crimsonsunset/mcp-mux/commit/4ddc339dae9b5210be87fc926774d83bf6491847))
* **planning:** propose Cursor agent hooks as a workspace signal ([6ba2c3d](https://github.com/crimsonsunset/mcp-mux/commit/6ba2c3deb3188165daa1e2cbeb49e3ae02ea2fbb))
* **planning:** reconcile connection-resilience docs and add leftovers inventory ([6cdba06](https://github.com/crimsonsunset/mcp-mux/commit/6cdba063062160b67bf1995130040f944ef638ad))
* **planning:** record PR [#8](https://github.com/crimsonsunset/mcp-mux/issues/8) architecture review and stale-doc pointers ([a86178a](https://github.com/crimsonsunset/mcp-mux/commit/a86178a5291730923a3f57264ec2051db7588307))
* record exact-call hook routing and Cloud Agent limits ([f03e7c7](https://github.com/crimsonsunset/mcp-mux/commit/f03e7c7da638e8a726f2abe603950f997a15b4e1))

## [0.5.0](https://github.com/mcpmux/mcp-mux/compare/v0.4.0...v0.5.0) (2026-06-25)


### Features

* per-workspace routing via X-Mcpmux-Workspace header + guided folder setup ([#182](https://github.com/mcpmux/mcp-mux/issues/182)) ([e2ec055](https://github.com/mcpmux/mcp-mux/commit/e2ec0558eada73407addc57902d9f13763cc8aec))
* **ui:** opencode global connect + client icons ([#184](https://github.com/mcpmux/mcp-mux/issues/184)) ([669e99f](https://github.com/mcpmux/mcp-mux/commit/669e99f2df3d85bbdd33427f0a96bd8e0048c7c3))


### Bug Fixes

* **gateway:** truly no-auth when inbound auth is disabled (no OAuth advertising) ([#187](https://github.com/mcpmux/mcp-mux/issues/187)) ([3e617fd](https://github.com/mcpmux/mcp-mux/commit/3e617fd876360f097fbffa96e21ce1fb90013fea))
* **oauth:** de-duplicate deep-link handling + quiet status-poll log ([#189](https://github.com/mcpmux/mcp-mux/issues/189)) ([9dd7b58](https://github.com/mcpmux/mcp-mux/commit/9dd7b58a23cef1f7f6a5ce53cab20750270c1a26))
* **storage:** drop a deleted FeatureSet from workspace bindings ([#186](https://github.com/mcpmux/mcp-mux/issues/186)) ([5598451](https://github.com/mcpmux/mcp-mux/commit/559845193aae3bc5bffff1e41758ffce1c083625))
* **ui:** scroll-to + flash the target Settings section on every redirect ([#190](https://github.com/mcpmux/mcp-mux/issues/190)) ([e032c9b](https://github.com/mcpmux/mcp-mux/commit/e032c9bc2e5a99ea64df4a4af215c5a29153af52))
* **ui:** show official opencode logo in the Apps tab ([#185](https://github.com/mcpmux/mcp-mux/issues/185)) ([608a841](https://github.com/mcpmux/mcp-mux/commit/608a841b1b0d058e055ff2ff0ba6cc37ed921a33))

## [0.4.0](https://github.com/mcpmux/mcp-mux/compare/v0.3.0...v0.4.0) (2026-06-19)


### Features

* [@mux](https://github.com/mux) UX + Windows updater fix + minimal-first optimization ([#171](https://github.com/mcpmux/mcp-mux/issues/171)) ([a215012](https://github.com/mcpmux/mcp-mux/commit/a215012ccd37388ffc6d802452e2fe03c9ce1ea5))
* **featureset:** protect Starter from deletion + clarify mapping popup ([#176](https://github.com/mcpmux/mcp-mux/issues/176)) ([163ee0b](https://github.com/mcpmux/mcp-mux/commit/163ee0b0ef0ac0166be0ecf9e2f8bad1612dfad3))
* **gateway:** default FeatureSet for unmapped roots + Mapped workspaces filter ([#175](https://github.com/mcpmux/mcp-mux/issues/175)) ([7fc50a0](https://github.com/mcpmux/mcp-mux/commit/7fc50a00923f74f752e5c279f60e232c1865c3e5))
* pre-release update channel + automated pre-releases from main ([#159](https://github.com/mcpmux/mcp-mux/issues/159)) ([e9306c4](https://github.com/mcpmux/mcp-mux/commit/e9306c4a8ac1aee72be2530697a643b69fb130f6))
* **spaces:** per-space base directories scope workspace roots to a Space ([#179](https://github.com/mcpmux/mcp-mux/issues/179)) ([fb825cf](https://github.com/mcpmux/mcp-mux/commit/fb825cfe66c6f383ea1f52d290a5519a67bffd5f))
* workspace-root routing + Tool Optimization ([@mux](https://github.com/mux)) self-management + UI live-sync ([#151](https://github.com/mcpmux/mcp-mux/issues/151)) ([d614853](https://github.com/mcpmux/mcp-mux/commit/d6148538b6f40644f9367d3c872bc1f4f2f7be63))
* **workspaces:** bulk-clear unmapped folders + clearer approval opt-out ([#172](https://github.com/mcpmux/mcp-mux/issues/172)) ([09b561c](https://github.com/mcpmux/mcp-mux/commit/09b561c901170cac2c6546ef8680b313f292eee6))
* **workspaces:** setting to disable the new-folder mapping prompt ([#177](https://github.com/mcpmux/mcp-mux/issues/177)) ([d5df002](https://github.com/mcpmux/mcp-mux/commit/d5df002df03e9b80a0bd780ce9c52fd9e942d02e))


### Bug Fixes

* add Windsurf, JetBrains, and Android Studio to quick-connect grid ([#139](https://github.com/mcpmux/mcp-mux/issues/139)) ([fb58d9c](https://github.com/mcpmux/mcp-mux/commit/fb58d9ce6c46ec1a55356a9fecb35f34ae2b29f6))
* **deps:** resolve 4 transitive security advisories failing Dependabot ([#174](https://github.com/mcpmux/mcp-mux/issues/174)) ([eb32289](https://github.com/mcpmux/mcp-mux/commit/eb32289ca7f309c52223f17cf3a1e1c0f0a61d7c))
* **gateway:** ride out self-update port race + clearer update restart UX ([#173](https://github.com/mcpmux/mcp-mux/issues/173)) ([6868992](https://github.com/mcpmux/mcp-mux/commit/6868992faeb77c8fe32ad7c996b2d196ca586002))
* **oauth:** DCR skip-invalid redirect URIs + drop duplicate RFC 8707 resource param ([#158](https://github.com/mcpmux/mcp-mux/issues/158)) ([661f162](https://github.com/mcpmux/mcp-mux/commit/661f1620105803acfe07087e997a1d4d00aa77d5))
* **servers:** pin config-modal footer ([#163](https://github.com/mcpmux/mcp-mux/issues/163)) + silent Windows updates ([#165](https://github.com/mcpmux/mcp-mux/issues/165)) ([0ddbdb5](https://github.com/mcpmux/mcp-mux/commit/0ddbdb59c7229f8bd9dd0a4875216af5ab8977af))
* **spaces:** clearer base-directories UX ([#180](https://github.com/mcpmux/mcp-mux/issues/180)) ([4a69908](https://github.com/mcpmux/mcp-mux/commit/4a699085087b775439180ace2467cda708b26fc3))
* **storage:** purge orphaned feature_set_members after the refactor (migration 017) ([#167](https://github.com/mcpmux/mcp-mux/issues/167)) ([b90b05c](https://github.com/mcpmux/mcp-mux/commit/b90b05c038d9d1bc8fae395e83e0e8db713d3e3f))


### Documentation

* add Discord community link to README ([#149](https://github.com/mcpmux/mcp-mux/issues/149)) ([c32f78f](https://github.com/mcpmux/mcp-mux/commit/c32f78f7143177589ea96b4e33170f49cc343b30))
* add Workspaces and Tool Optimization guides ([#164](https://github.com/mcpmux/mcp-mux/issues/164)) ([25a6fbb](https://github.com/mcpmux/mcp-mux/commit/25a6fbb94d3efcbd0cd5a714ac452c5e61608250))
* complete the getting-started flow + workspace-driven routing ([#166](https://github.com/mcpmux/mcp-mux/issues/166)) ([92f8ac2](https://github.com/mcpmux/mcp-mux/commit/92f8ac2f053e27b4a1aec222eea7ff3f9986559c))

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
