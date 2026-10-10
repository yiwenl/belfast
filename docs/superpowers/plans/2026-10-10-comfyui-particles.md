# ComfyUI 粒子配色 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking. 第一版已實作；以下保留原工作拆分，驗證紀錄見末段。

**Goal:** 複製目前網頁粒子範例，讓使用者輸入英文 prompt，取得 ComfyUI 圖片並即時更新每顆粒子的配色。

**Architecture:** Vite 代理 HTTP／WebSocket，獨立 client 處理一次生成與取消清理。以固定的 instance UV 取樣圖片，透過既有 Belfast Texture API 切換圖片；擴充 WASM 的相機＋陰影＋圖片資源組合。

**Tech Stack:** TypeScript、Vite 6、Belfast Rust／WASM、wgpu 30、WGSL、ComfyUI HTTP／WebSocket。客戶端測試採 `node:test` 與 `tsx`，不更換現有框架。

**Spec:** [設計草案](../specs/2026-10-10-comfyui-particles-design.md)。實作依此範圍完成，例外與驗證限制列於末段。

## Global Constraints

- 新範例路徑為 `/particles-image/`，原 `/particles/` 保留。
- 粒子數維持 `1_000_000`，模擬資料 stride 維持 `48` bytes。
- 保留現有 compute 與 shadow shader；新增 UV 不改動運動資料。
- 每顆粒子的 UV 在初始化時隨機產生一次，範圍為 `[0, 1)`，生成新圖片時保持不變。
- API 使用 `512×512`、batch `1`、steps `1`；每次產生新 seed。
- 首次尚未成功的生成使用 `120000 ms` timeout，之後使用 `30000 ms`；取消清理可另花最多約 `6000 ms`。
- 每次手動送出一筆；忙碌期間鎖住輸入與送出按鈕，動畫持續。
- 成功時直接切換圖片；失敗保留目前貼圖並允許手動重試。
- 初始圖片為 `1×1` 純白貼圖；最終顏色為圖片 RGB × 原粒子灰階 × 現有陰影亮度，因此初始畫面保有目前灰階與陰影。
- 保留工作目錄既有未提交修改；所有提交只包含本次變更。

## Review Focus

1. WebSocket 圖片／完成事件早於 POST 回應：仍正確完成，忽略其他 prompt 與 preview。由 Task 3 測試。
2. 逾時／abort 發生在工作可能已送出之後：只取消自己的 prompt，清理結束才解鎖，不自動重送。由 Task 3 測試。
3. 新圖片解碼或 bind group 建立失敗：保留舊圖並釋放候選資源。由 Task 2 測試。
4. 頁面卸載後遲到的圖片／錯誤：不得使用已釋放的 device 或更新表單。由 Task 2、4 測試。
5. 三個 bind groups 數量正確但編號重複、缺漏或屬於別的 draw：在編碼前拒絕，既有雙 group 功能仍通過。由 Task 1 測試。

## 檔案與責任

| 路徑                                                        | 工作                                                |
| ----------------------------------------------------------- | --------------------------------------------------- |
| `web/particles-image/index.html`                            | 新範例 canvas 與表單容器                            |
| `web/particles-image/src/main.ts`                           | 複製目前粒子範例，組裝 renderer、UV、表單與生命週期 |
| `web/particles-image/src/shaders/*.wgsl`                    | 複製三個 shader，只修改可見粒子的取樣著色           |
| `web/particles-image/src/image-texture.ts`                  | 白圖、PNG 解碼、texture／bind group 替換與回收      |
| `web/particles-image/src/api-client.ts`                     | 移植範例 client，HTTP／WS、請求識別、錯誤與取消     |
| `web/particles-image/src/workflow-api.json`                 | API 範例原始 workflow，逐次複製後調整               |
| `web/particles-image/src/prompt-panel.ts`                   | 表單狀態、提交鎖定與可讀錯誤                        |
| `web/particles-image/src/style.css`                         | 此範例專用表單樣式                                  |
| `web/particles-image/tests/*.test.ts`                       | API、資源回收及表單非同步狀態測試                   |
| `web/vite.config.ts`、`web/tsconfig.json`、`web/index.html` | 代理、build entry、型別檢查範圍與入口連結           |
| `web/.env.example`                                          | `COMFY_URL=http://192.168.1.201:8189` 範例          |
| `web/package.json`、`pnpm-lock.yaml`                        | `tsx` 測試相依與 `test:particles-image` 指令        |
| `crates/belfast-wasm/src/draw.rs`                           | 新 shader 資源組合及驗證                            |
| `crates/belfast-wasm/src/bind_group.rs`                     | 新組合的 uniform／shadow／texture 驗證              |
| `crates/belfast-wasm/src/frame.rs`                          | 三組資源完整性驗證                                  |
| `crates/belfast/tests/example_shaders.rs`                   | 新 shader 的 wgpu 驗證                              |
| `web/particles-image/README.md`                             | 本機啟動、API 設定與驗收方式                        |

## Task 1：支援相機、陰影與圖片一起綁定

**Files:** 修改 `crates/belfast-wasm/src/{draw,bind_group,frame}.rs`，測試放各檔現有 test module。

**Interfaces:**

- 新增 `ShaderResourceLayout::UniformShadowAndTexture`，欄位明確分開 uniform、shadow 與 image 的 group／binding；`bind_group_count()` 回傳 `3`。
- 固定 group `0` uniform binding `0`；group `1` depth／comparison binding `0/1`；group `2` sampled image／filtering sampler binding `0/1`。
- 延用 `BindGroup.fromBuffer`、`BindGroup.fromShadowMap` 與 `BindGroup.fromTexture(..., { groupIndex: 2 })`。
- 延用 `frame.render(draw, [cameraGroup, shadowGroup, imageGroup], count)`。

- [x] 寫失敗測試：三資源 shader 能通過資源及 vertex／fragment 介面檢查；texture 在 vertex 使用 `textureSampleLevel`。錯誤 group、sampler 種類、缺漏資源及 depth-only 使用新組合都被拒絕。
- [x] 補 binding／frame 測試：接受完整 `0/1/2`；拒絕 `0/1/1`、少一組、跨 device、跨 draw；保留既有單組及 `0/1` 測試。
- [x] 執行 `cargo test -p belfast-wasm --lib`，確認新測試因目前不支援三資源而失敗。
- [x] 擴充上述三檔的 enum、match 與驗證。Frame 驗證 group 集合完整性，不僅檢查數量；不擴張成任意 shader 資源系統。
- [x] 執行 `cargo test -p belfast-wasm --lib` 與 `cargo check -p belfast-wasm --target wasm32-unknown-unknown`；全部通過後僅提交本次修改。

## Task 2：複製範例並打通固定圖片配色

**Files:** 新增 `web/particles-image/index.html`、`src/main.ts`、`src/shaders/*.wgsl`、`src/image-texture.ts`、`tests/image-texture.test.ts`；修改網頁 entry、tsconfig、package、lockfile 與 `crates/belfast/tests/example_shaders.rs`。

**Interfaces:**

- `ParticleImageTexture.create(device: Device, draw: Draw): Promise<ParticleImageTexture>`：建立白圖與 group `2`。
- `ParticleImageTexture.bindGroup: BindGroup`：renderer 每幀讀取目前有效 binding。
- `ParticleImageTexture.replace(blob: Blob): Promise<void>`：新資源完成後原子替換，失敗保留有效資源。
- `ParticleImageTexture.dispose(): void`：可重複清理，拒絕遲到結果。

- [x] 複製當前 `web/particles/` 至新目錄；加入首頁連結、build entry 與 TypeScript include。保持原範例及未提交修改。
- [x] 在 web devDependencies 加 `tsx`，新增 `test:particles-image`：`node --import tsx --test particles-image/tests/*.test.ts`。資源單元測試使用假的 Belfast 工廠與 bitmap 解碼器，不需要真實 GPU。
- [x] 寫失敗測試：初始白圖可綁定；替換成功才釋放舊組合；解碼或 binding 建立失敗仍保有舊組合；bitmap 在各路徑 close；dispose 期間完成的解碼不會新增 GPU 資源；dispose 可重複呼叫。
- [x] 執行 `pnpm --filter @belfast/web-examples test:particles-image` 確認失敗，再實作 `ParticleImageTexture`。以窄介面注入資源工廠供測試，正式環境仍呼叫既有 Belfast API。
- [x] 在既有粒子初始化結束後建立 UV buffer；使用 slot `2`、location `3`、stride `8` 的 per-instance `vec2`，不改 48-byte storage buffer 或任何運動參數。
- [x] 可見 shader 加 group `2` 圖片資源與 UV 輸入；vertex 執行 `textureSampleLevel(..., 0.0).rgb * instance_color.xyz`。fragment、compute、shadow 的既有行為保留。
- [x] render 每幀使用最新 `imageTexture.bindGroup`。錯誤圖片的處理由操作流程捕捉，不停止動畫。清理新增 buffer、texture 與 binding。
- [x] 新增 wgpu shader 測試，檢查新 draw shader（含 `wgslShadowPcf3x3()` 前綴）、compute 與 shadow shader；若無 adapter 必須明確記錄 skip，再由瀏覽器驗證補足。
- [x] 執行資源測試、`cargo test -p belfast --test example_shaders`、`pnpm build:wasm`、`pnpm typecheck` 與 web build。用開發用固定四色 fixture 檢查配色、方向、陰影與白圖效果，量測與原例的幀時間，再提交此階段。

## Task 3：加入 ComfyUI client 與同來源代理

**Files:** 新增 `src/api-client.ts`、`src/workflow-api.json`、`tests/api-client.test.ts`、`web/.env.example`；修改 `web/vite.config.ts`。

**Interfaces:**

- `new ParticleImageClient({ baseUrl, workflow, fetch?, WebSocket? })`：兩個可選 transport 用於測試；正式 `baseUrl` 字串由 `new URL('/comfy', window.location.origin).href` 取得。
- `Workflow = Record<string, { class_type: string; inputs: Record<string, unknown> }>`；`ComfyEvent = { type: string; data: { prompt_id?: string; node?: string | null; [key: string]: unknown } }`。處理伺服器 JSON 時先檢查必需欄位，再縮小型別。
- `generate(text: string, options: { timeoutMs: number; signal?: AbortSignal; seed?: number; onEvent?: (event: ComfyEvent) => void }): Promise<GeneratedImage>`。
- `GeneratedImage = { blob: Blob; promptId: string; seed: number; steps: number; receivedMs: number; completedMs: number }`；steps 固定 `1`。
- `busy: boolean`：涵蓋請求與取消清理期間；每次生成複製 workflow，不改原始 JSON。

- [x] 將指南中的 workflow 原樣加入專案；依已讀過的 `client.mjs` 移植 TypeScript，保留來源連結與完成事件相容行為。
- [x] 先寫測試 fixtures：假的 fetch、WebSocket 及短 timeout，模擬送出前 WS listener 已存在、事件早於 POST 回應、只有正確 prompt／node `9` PNG 被接受、完成但無 PNG 則失敗。
- [x] 測試空白輸入、重複請求、queue busy、HTTP 400、回傳不同 prompt ID、執行錯誤、socket 中斷、逾時與外部 abort。前置失敗不得送 prompt；提交後的不確定狀態仍清理本次 ID。
- [x] 測試取消依序為 `/queue {delete:[id]}`、`/interrupt {prompt_id:id}`、再次刪除 queue；任一步失敗仍嘗試下一步，每步 `2000 ms` 上限。取消未確認需帶可辨識資訊，busy 在 finally 恢復；沒有自動 retry。
- [x] 執行 `pnpm --filter @belfast/web-examples test:particles-image` 確認失敗，再實作 client。檢查 signal 已先 abort、WS 失敗早於 open 及 Promise 拒絕都能結束等待。
- [x] Vite 以 `loadEnv` 讀取 `COMFY_URL`，加 `/comfy` 代理、去前綴、`ws: true`、`changeOrigin: true`。維持 localhost listener；檢查 query string 的 client ID 未被 rewrite 丟失。
- [x] 執行 client 測試、typecheck 與 web build。瀏覽器透過代理讀 `/system_stats`／`/queue` 並建立、關閉 WS，確認上游 Origin 相容；這一步不提交生成工作。成功後提交此階段。

## Task 4：串接表單、錯誤恢復與生命週期

**Files:** 新增 `src/prompt-panel.ts`、`src/style.css`、`tests/prompt-panel.test.ts`；修改 `src/main.ts`、`index.html`。

**Interfaces:**

- `mountPromptPanel(root: HTMLElement, handlers: { generate(text: string, signal: AbortSignal, timeoutMs: number): Promise<GeneratedImage>; applyImage(blob: Blob): Promise<void> }): { dispose(): void }`。
- 狀態固定為 `idle | generating | applying | ready | error`；內部持有當前 AbortController 與 dispose／request guard。
- `prompt-panel.ts` 管理首次成功旗標，選擇 `120000`／`30000` timeout，並將 client 結果交給 `ParticleImageTexture.replace`。

- [x] 寫行為測試：空白 prompt 不執行；兩次 submit 只呼叫一次；等待 generate 與 apply 的整段期間禁用表單；兩階段任何拒絕都保留 prompt 並解鎖；dispose abort，遲到結果不呼叫 apply 或更新 UI。
- [x] 測試首次尚未成功採 `120000 ms`，成功後採 `30000 ms`；錯誤不觸發自動重送。
- [x] 使用小型可替換 view 介面測試非同步狀態，瀏覽器驗證實際 DOM 行為；不要為表單引入 UI 框架。先執行測試確認失敗，再實作與串接。
- [x] 加入英文 prompt 提示、送出按鈕、`aria-live` 狀態與錯誤訊息。error 用 `textContent`，表單事件不傳給 canvas 的相機控制。
- [x] busy 期間保持 requestAnimationFrame 運作。API 可恢復錯誤與既有渲染致命錯誤分開處理；不以大面積堆疊資訊蓋住粒子。
- [x] 同一份冪等清理函式處理頁面離開與 HMR：先取消表單請求並封鎖新工作，再關閉 renderer 與資源；late bitmap 只 close，不操作 freed device。
- [x] 執行所有客戶端測試、typecheck 與 web build；手動驗證鍵盤輸入、送出、忙碌和錯誤狀態，再提交此階段。

## Task 5：端到端驗收與說明文件

**Files:** 新增 `web/particles-image/README.md`；必要時修正前述檔案。

**Interfaces:** `pnpm dev` 後進入 `/particles-image/`，由使用者手動送出英文 prompt。

- [x] 用一筆英文描述完成真正生成，記錄送出、PNG 收到、解碼完成、新 binding 首次提交繪製的時間。服務端文件延遲不當作 Mac 的驗收保證。
- [x] 在 API 等待與錯誤時確認粒子運動與相機仍持續；換圖不呼叫 spawnParticles、不更新 UV buffer、不重建 simulation。
- [x] 驗證斷線、server busy、逾時、錯誤圖片及連點；取消只影響本次 prompt。模擬 transport 錯誤時使用測試 client，避免干擾其他主機工作。
- [x] 連續手動生成十次，確認成功換色與無 console 錯誤；以單元測試確認資源、socket／listener 回收。比較原範例與新範例畫面更新間隔，記錄解碼上傳耗時。
- [ ] 長時間 GPU 記憶體趨勢與 GPU timestamp profiling 尚未量測；目前驗證涵蓋有限次真實生成與生命週期測試。
- [x] 跑最終檢查：`cargo test -p belfast-wasm --lib`、`cargo test -p belfast --test example_shaders`、`cargo check -p belfast-wasm --target wasm32-unknown-unknown`、`pnpm --filter @belfast/web-examples test:particles-image`、`pnpm build`、`pnpm typecheck`、`git diff --check`。僅格式化本次改動。
- [x] 在瀏覽器驗證原 particles 與 basic texture 範例無回歸，新範例無 WGSL／WebGPU validation error。沒有 GPU 的測試環境不能替代這項驗收。
- [x] README 說明 `COMFY_URL`、localhost／WebGPU、英文 prompt、首次等待較長、手動重試、單一呼叫端前提，以及正式部署需要另一個可達區網的代理。提交說明與已驗證的修正。

## 後續擴充

第一版驗收後，再評估雙 texture 漸變、圖片預覽與 seed 控制。雙 texture 漸變需要另定貼圖占用及替換時機；不預先把這些狀態帶進 MVP。

## 實作與驗證紀錄（2026-10-10）

- 保留在既有 `particles` 分支；已有使用者未提交的修改，未自動建立 commit 或合併。上述步驟的「提交」改為保留可檢視的工作目錄差異。無關的 11 個原有修改檔案與基線逐位元一致。
- Rust WASM 101 項測試、原生 GPU shader 3 項測試、客戶端 26 項測試通過；workspace 測試、wasm32 check、WASM／web build、typecheck 通過。
- 首次成功逾時選擇由表單 controller 管理，便於覆蓋整段序列操作。
- 根據實際 `Bgra8Unorm` 畫布，圖片採 `rgba8unorm` 保留原 shader 色彩空間；sRGB 畫布才採 sRGB texture。
- Chrome 真實生成連續十次成功（0.6–0.9 秒）；固定四色圖、新舊粒子與既有 texture 範例均可顯示且無 GPU validation error。
- 實測 BFCache 返回會還原已停用的舊頁面，已加入 persisted pageshow 重新載入並驗證恢復可操作。
- 效能與量測限制見 [README 驗收紀錄](../../../web/particles-image/README.md)。
