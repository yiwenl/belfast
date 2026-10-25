# 圖片配色粒子

輸入英文場景描述，從區域網路的 ComfyUI 取得圖片，將圖片配色套用到一百萬顆粒子。每顆粒子的 UV 固定且隨機，換圖只改顏色，保留原本的運動與陰影。

左下角面板會顯示目前配色使用的生成原圖與擷取色票；成功套用新配色後一起更新，等待或失敗時保留上一組。滑鼠移到色票上可查看 HEX 色碼。

## 開啟

在專案根目錄執行：

```sh
pnpm dev
```

在支援 WebGPU 的瀏覽器開啟 [粒子配色頁面](http://localhost:5173/particles-image/)。初始為白色貼圖；API 主機未連線也能先看到粒子。

輸入英文描述後按「生成配色」，或使用 Cmd／Ctrl＋Enter。例如：

```text
A bright coral reef, turquoise water, vivid orange coral, golden sunlight
```

每次只處理一筆。生成與套用期間表單暫停輸入，相機及動畫持續；失敗保留目前配色與輸入文字，可手動再試。第一次成功前最多等待 120 秒，之後最多 30 秒；失敗後定向取消清理可能另需約 6 秒。沒有自動重試或背景生成。

## 色彩增強後綴

每次送出生成前，會從 `src/prompt-enhancement.ts` 的 `COLOR_PROMPT_SUFFIXES` 隨機選一個短後綴，加在原始描述後面。內容著重明亮中間色、清楚的色彩對比，以及少量輔助色變化；不指定新的物件或固定主色。輸入框仍保留原文，重複送出不會累積後綴；隨機選取允許重複。

後綴使用 ComfyUI 的 `(描述:權重)` 語法，`COLOR_PROMPT_WEIGHT` 預設為 `0.65`，降低增強描述的影響，讓使用者指定的場景與主色保持優先。若效果太強可再調低。這是生成提示，無法保證每張圖片都有固定的亮度或色彩分布。Console 的 `effectivePrompt` 會記錄這次實際送出的完整文字，方便調整。

## 圖片色票

圖片回傳後，前端將取樣圖縮至最長邊 128 像素，以顏色直方圖與依像素數加權的 k-means 擷取最多 8 個代表色；接近的顏色合併，純色圖只會得到一個色票。不依賴模型畫出調色盤，也不增加 API 呼叫。色票數量可調整 `src/palette.ts` 的 `PALETTE_SIZE`（1～16）。

代表色依明暗排序，製作成 N×1 貼圖，每個色票占一個像素。採 nearest 取樣，避免色票間插值；固定的隨機 UV 讓每個色票分配到約相同數量的粒子，使小面積輔助色有機會更明顯。色票保留擷取的 RGB，沒有額外提高亮度或飽和度；粒子仍會受原有灰階、shader 調色及陰影影響。全透明或無法解碼的圖片會保留上一組配色並顯示錯誤。

## 顏色漸變

新的色票貼圖在 A／B 兩組貼圖間交替寫入，保留上一組作為漸變起點。每顆粒子初始化時產生固定的 UV 與 `rnd`（0～1），依 `blend = clamp(offset * 2 - rnd, 0, 1)` 混合兩組色票，因此換色會錯開開始，結束時所有粒子都到達新配色。

`offset` 在 2 秒內由 0 增加到 1，可修改 `src/image-texture.ts` 的 `COLOR_TRANSITION_SECONDS` 調整時長。漸變完成後才更新原圖預覽並解鎖表單；狀態列的總耗時包含生成與漸變。A／B 貼圖使用 group 2／3，offset 和目標槽位附加於原相機 uniform，粒子模擬與陰影維持不變。

已在 Chrome 實測白色→花田、花田→海洋兩次交替，漸變約 2 秒，無 WebGPU／console 錯誤。104 項 WASM 測試與 27 項客戶端測試通過，涵蓋四組 binding 完整性、交替方向、失敗保留原圖與資源回收。

## API 設定

預設連至 `http://192.168.1.201:8189`。更換主機時，將 `web/.env.example` 複製為 `web/.env.local`，設定：

```dotenv
COMFY_URL=http://192.168.1.201:8189
```

重新啟動開發伺服器即可。瀏覽器使用同來源 `/comfy` 路徑，由 Vite 代理 HTTP 與 WebSocket；不需要從瀏覽器直接跨來源連線。開發服務固定為 localhost:5173，若連接埠被占用，先停止原本服務。

Windows 服務須開機並可從這台電腦的區域網路連線。此範例使用指南附帶的 SD-Turbo workflow，512×512、batch 1、steps 1，透過節點 9 的 WebSocket PNG 取得結果。服務目前以單一呼叫端使用為前提；queue 檢查不是多台電腦之間的原子鎖。

若顯示取消未確認，先確認伺服器狀態再重試。取消只指定自己的 prompt ID；關閉整個瀏覽器時不能保證非同步取消完成。

WebGPU 需要 localhost 或安全來源。`pnpm build` 只產生靜態檔案，不包含 Vite 開發代理；正式部署需配置可連到該區網主機的 `/comfy` HTTP／WebSocket 反向代理，以及合適的 HTTPS 來源。

## 驗證

```sh
pnpm --filter @belfast/web-examples test:particles-image
pnpm typecheck
pnpm build
cargo test -p belfast-wasm --lib
cargo test -p belfast --test example_shaders
```

客戶端測試涵蓋提早抵達的 WebSocket 結果、preview 過濾、queue busy、HTTP／socket 錯誤、逾時與 abort、定向取消、表單鎖定，以及失敗／卸載時的圖片資源回收。原有 `/particles/` 與 `/basic/` 可用於回歸比較。

瀏覽器 console 的 `[particles-image]` 紀錄包含 PNG 收到、生成完成、解碼上傳及漸變完成的 frame 提交時間。提交 frame 不等同螢幕完成顯示。圖片格式依 canvas 決定：unorm 畫布保留圖片的編碼 RGB，sRGB 畫布則使用 sRGB texture 解碼，避免重複造成暗化。

## 初版直接換色驗收紀錄（2026-10-10）

Chrome、1728×941 視窗、devicePixelRatio 2、一百萬顆粒子：

- 白圖與固定四色圖顯示正常，保留陰影；原 particles 與 basic texture 範例無回歸。
- 連續十次真實生成皆成功，介面顯示 0.6–0.9 秒；其中一次 PNG 收到約 678 ms，解碼上傳 7.8 ms，新配色 frame 首次提交 8.8 ms。
- 同一瀏覽器以暫時測試頁記錄各 240 次 rAF 更新間隔：原範例平均 10.76 ms、p95 16.9 ms；圖片版平均 12.67 ms、p95 18.1 ms。這是畫面排程間隔，受隨機粒子分布與系統負載影響，不是 GPU 執行時間或正式 benchmark。
- 26 項客戶端測試通過，涵蓋資源釋放、bitmap close、socket／listener 清理及錯誤恢復；十次真實生成未出現 console／WebGPU 錯誤。尚未量測長時間 GPU 記憶體趨勢。
- 返回瀏覽器快取頁面時，會重新載入以重建已釋放的 GPU 資源，表單可繼續使用；先前配色不跨頁面重載保存。

## 來源

- [本機 API 指南](http://192.168.1.201:8189/api-guide.html)
- [原始 client 與 workflow 套件](http://192.168.1.201:8189/particle-api-starter.zip)
- [設計文件](../../../docs/superpowers/specs/2026-10-10-comfyui-particles-design.md)
- [實作計畫](../../../docs/superpowers/plans/2026-10-10-comfyui-particles.md)
