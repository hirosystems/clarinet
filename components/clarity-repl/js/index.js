// @ts-check

import request from "sync-request";

const encoder = new TextEncoder();

/**
 * httpClient
 * @param {import("sync-request").HttpVerb} method
 * @param {string} path
 * @param {import("sync-request").Options} options
 * @returns {Uint8Array}
 */
export function httpClient(method, path, options) {
  const response = request(method, path);
  if (typeof response.body === "string") {
    return encoder.encode(response.body);
  }
  return response.body;
}
