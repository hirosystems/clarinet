// @ts-check

import request from "sync-request";

const encoder = new TextEncoder();

export function getHiroApiKey() {
  const isNode = typeof process !== "undefined" && process.env != null;
  if (!isNode) return undefined;
  return process.env.HIRO_API_KEY;
}

/**
 * httpClient
 * @param {import("sync-request").HttpVerb} method
 * @param {string} path
 * @returns {Uint8Array}
 */
export function httpClient(method, path) {
  const options = {
    headers: {
      "x-hiro-product": "clarinet-sdk",
    },
  };
  const apiKey = getHiroApiKey();
  if (apiKey) {
    options.headers["x-api-key"] = apiKey;
  }

  const response = request(method, path, options);
  if (typeof response.body === "string") {
    return encoder.encode(response.body);
  }
  return response.body;
}
