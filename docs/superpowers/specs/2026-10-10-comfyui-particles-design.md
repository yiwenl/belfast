# ComfyUI 圖片驅動粒子配色設計草案

以目前 `web/particles` 的工作目錄版本建立獨立的 `web/particles-image` 範例。輸入英文描述後，透過區域網路的 ComfyUI 取得圖片，再用每顆粒子固定的隨機 UV 取樣圖片作為配色。保留現有一百萬顆粒子的運動、相機、陰影及輔助圖形。

第一版已於 2026-10-10 實作；實測結果與限制見 [範例 README](../../../web/particles-image/README.md)。

## 範圍與預設

- 新範例路徑為 `/particles-image/`，原 `/particles/` 保留。
- 粒子數維持 `1_000_000`，模擬資料 stride 維持 `48` bytes。
- 保留現有 compute 與 shadow shader；新增 UV 不改動運動資料。
- 每顆粒子的 UV 在初始化時隨機產生一次，範圍為 `[0, 1)`，生成新圖片時保持不變。
- 初始圖片為 `1×1` 純白貼圖；最終顏色為圖片 RGB × 原粒子灰階 × 現有陰影亮度，因此初始畫面保有目前灰階與陰影。
- 每次手動送出一筆；忙碌期間鎖住輸入與送出按鈕，動畫持續。
- API 使用 `512×512`、batch `1`、steps `1`；每次產生新 seed。
- 首次尚未成功的生成使用 `120000 ms` timeout，之後使用 `30000 ms`；取消清理可另花最多約 `6000 ms`。
- 成功時直接切換圖片；失敗保留目前貼圖並允許手動重試。
- MVP 不加入圖片漸變、粒子重組成圖片、中文翻譯、背景連續生成、多請求佇列或生成歷史。
- 開發頁面使用支援 WebGPU 的瀏覽器與 `http://localhost:5173`；伺服器 IP 由 `COMFY_URL` 設定。

## 實作前的現況

`web/particles/src/main.ts` 目前透過 `@belfast/wasm` 建立粒子，使用同一個 storage/vertex buffer 進行 compute 更新與 instanced quad 繪製。`particles_update.wgsl` 使用 `velocity.w` 與 `color.w` 計算力量；這些欄位不能拿來存 UV。

`Texture.fromImageBitmap` 已存在，預設採用 `rgba8unorm-srgb`、線性取樣及 `flipY: true`。已有基本圖片範例可參照，無須為第一版新增另一套原生 WebGPU renderer。

WASM 的 `ShaderResourceLayout` 現在只接受無資源、uniform、圖片＋sampler、uniform＋shadow 四種組合。`BindGroup` 驗證也依賴這些組合，`Frame` 目前只對兩個 group 驗證完整 group 編號。新增圖片與既有相機、陰影共存，必須同步擴充這三處。

目前工作目錄另有未提交修改，包括粒子主程式與 WASM binding。實作時以當時工作中的版本為基礎複製，保留既有修改，避免把這次工作與其他變更混合提交。

## API 與連線方式

已讀取使用者提供的 [API 指南](http://192.168.1.201:8189/api-guide.html)，並檢查其 [範例套件](http://192.168.1.201:8189/particle-api-starter.zip) 內的 `client.mjs` 與 `workflow-api.json`。這是 SD-Turbo 的 HTTP＋WebSocket 工作流程：

1. `GET /queue`，伺服器有工作時回報忙碌。
2. 每次請求建立不同的 client ID 與 prompt ID，先連線 `/ws?clientId=…` 並準備事件接收。
3. 複製 workflow，設定節點 `6` 的文字、節點 `3` 的 seed、節點 `10` 的 steps，再 `POST /prompt`。
4. 只追蹤符合本次 prompt ID 的 JSON 事件；只有正在執行輸出節點 `9` 時，才接受 event `1`、format `2` 的 PNG binary frame，去除其 8-byte header。
5. 收到 PNG 且工作完成才 resolve。範例 client 同時接受 `execution_success` 或符合本次 prompt 的 `executing: null` 完成事件；移植時保留並測試這項相容行為。
6. 失敗或逾時時，若請求已送出，僅針對本次 prompt 刪除 queue 項目及 interrupt，清理完成才解除 busy。

`POST /prompt` 的回應不是圖片，WebSocket 事件可能先於 HTTP 回應抵達；二進位 frame 也不帶 prompt ID。不可用一般單一 fetch 圖片的方式取代上述流程。既有 client 的流程可作為 TypeScript 移植基礎，補上頁面銷毀時的 `AbortSignal` 支援及可測試介面。

建議採用 **Vite 同來源代理**：瀏覽器只連 `/comfy/*`，Vite 去除前綴後轉送到 `COMFY_URL`，同時代理 WebSocket。範例設定預設目標為 `http://192.168.1.201:8189`。需在整合時實測 WebSocket Origin 是否被上游接受；若被拒絕，針對實際 localhost origin 調整上游設定，不加入通配 CORS 或全面改寫 Origin。

另外兩種方案是直接開放指定 origin 的 CORS，或把前端與 API 放在同一個正式反向代理後。前者需要修改 Windows 設定，後者適合後續部署。第一版沿用本機 Vite，工程量較小。Vite 開發代理不會包含在靜態 build 輸出；之後部署仍需能連到區網主機的 HTTP／WebSocket 代理。WebGPU 需要安全來源；其他裝置直接開一般區網 HTTP 頁面不能視為等同 localhost。

## GPU 資料與著色

新增獨立的 instance UV vertex buffer：每顆 `vec2<f32>`，stride `8` bytes，`slot: 2`、`shaderLocation: 3`、`stepMode: "instance"`。一百萬顆需新增 `8,000,000` bytes GPU 資料，約 `7.6 MiB`；CPU 初始化陣列另占相同大小，可在上傳後解除參照。

先完成原本的 `spawnParticles()`，再產生 UV，避免額外的 random 呼叫穿插進既有粒子初始化。UV 不加入 compute binding，也不在每幀更新。

繪製資源固定為：

| Group | 資源                                                   |
| ----- | ------------------------------------------------------ |
| `0`   | 原有相機 uniform，binding `0`                          |
| `1`   | 原有 shadow texture／comparison sampler，binding `0/1` |
| `2`   | 新圖片 texture／filtering sampler，binding `0/1`       |

在 vertex shader 以 `textureSampleLevel(imageTexture, imageSampler, particleUv, 0.0)` 取得 RGB，再乘上原灰階，傳給 fragment shader。各頂點使用相同 instance UV，使每顆 billboard 呈現一致配色。fragment shader 保留圓形裁切及陰影；shadow shader 不讀圖片。

WASM 新增 `UniformShadowAndTexture` 資源組合，延續目前有限且明確的 layout 模式。更新 shader 檢查、三種 `BindGroup` 工廠的驗證，以及 `Frame` 對三個 group 的完整性檢查；維持既有範例與 API 行為。

## UI 與狀態

canvas 上疊加小型表單，包含多行文字框、送出按鈕，以及同一個狀態／錯誤區域。提示文字說明目前以英文描述色彩、光線與場景；使用 `textContent` 顯示伺服器訊息。表單不接收或傳遞相機拖曳操作。

狀態為 `idle → generating → applying → ready`；任一步驟失敗進入 `error`。`generating` 可顯示連線、生成或取消清理階段的文字，沒有可靠進度時不顯示虛構百分比。`generating` 與 `applying` 都算 busy。

空白輸入不送出；重複 submit 在 UI 與 client 兩層擋住。首次成功前使用較長等待期限，不自動送出暖機請求。錯誤後保留 prompt 方便修改。

## 圖片切換與資源生命週期

以 `createImageBitmap(blob)` 解碼，呼叫既有 `Texture.fromImageBitmap`，再建立 group `2` 的 bind group。初始白圖也走同一路徑，可用白色 `ImageData` 建立 bitmap。圖片格式依實際畫布選擇：瀏覽器 unorm 畫布使用 `rgba8unorm`，sRGB 畫布使用 `rgba8unorm-srgb`，沿用原 shader 的色彩處理，避免圖片暗化。

新 texture 與 bind group 都建立成功後，才一次替換 renderer 的有效資源；舊圖維持到新圖準備好。失敗則釋放候選資源，保留舊圖。bitmap 在成功與失敗路徑都 `close()`。舊 bind group 與 texture 在不再由後續 frame 使用後依序 `free()`，沿用 Belfast 內部的 GPU 參照保留機制。

頁面離開或 HMR dispose 時停止動畫、移除事件監聽、abort 本次工作並防止遲到結果更新畫面。若從瀏覽器 BFCache 返回，重新載入頁面以重建已釋放的 GPU 資源。HTTP 提交後的 abort 仍須做本次 prompt 的定向清理；瀏覽器完全關閉時無法保證非同步清理完成，重開後仍以 `/queue` 檢查為準。正在解碼的 bitmap 若在 dispose 後才完成，立即 close，不再操作已釋放的 device。

API、圖片解碼與 texture 更新錯誤屬於可恢復操作錯誤，不進入現有 render loop 的致命錯誤路徑。真正 device／render 錯誤另行顯示。

## 驗收條件

- 新舊範例都可開啟；新範例在無 API 連線時仍能先顯示原本灰階粒子與運動。
- 固定四色測試圖可產生四色粒子，保留陰影；白色圖可恢復原先灰階效果。
- 在同一瀏覽器、相同粒子設定下比較原範例與新範例幀時間；記錄圖片上傳時的短暫變化。
- 送出一次英文 prompt 後，收到實際 PNG 並換色，UV 與粒子狀態不重新初始化。
- 等待 API 時畫面與相機操作持續；連點只送一筆。
- 伺服器 busy、HTTP 錯誤、斷線、逾時、取消未確認、錯誤圖片及資源建立失敗，都保留上一張配色並恢復可操作狀態。
- 重複生成與頁面重載不累積有效 texture、bitmap、socket 或 listener。
- 既有 texture、particles、相機與陰影範例沒有 binding 回歸；shader、TypeScript 與 WASM 檢查通過。

## 參考

- [Vite server.proxy](https://vite.dev/config/server-options.html#server-proxy)：HTTP／WebSocket 代理。
- [MDN WebGPU GPU](https://developer.mozilla.org/en-US/docs/Web/API/GPU)：安全來源條件。
- [WGSL textureSampleLevel](https://www.w3.org/TR/WGSL/#texturesamplelevel)：指定 mip level 的圖片取樣。
