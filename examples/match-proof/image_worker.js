import { runImageRequest } from "./browser.js";

self.onmessage = async ({ data: { request, references } }) => {
    try {
        const result = await runImageRequest(request, references);
        self.postMessage({ result });
    } catch (error) {
        self.postMessage({ error: error.stack || String(error) });
    }
};
