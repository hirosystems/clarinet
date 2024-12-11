import request, { HttpVerb, Options as HttpOptions } from "sync-request";

const encoder = new TextEncoder();

export function httpClient(method: HttpVerb, path: string, options?: HttpOptions): Uint8Array {
  const response = request(method, path, options);
  if (typeof response.body === "string") {
    return encoder.encode(response.body);
  }
  return response.body;
}
