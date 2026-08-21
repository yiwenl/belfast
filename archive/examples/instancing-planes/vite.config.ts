import { createExampleConfig } from "../shared/vite.config.base";
import path from "node:path";
import { fileURLToPath } from "node:url";

export default createExampleConfig(path.dirname(fileURLToPath(import.meta.url)));
