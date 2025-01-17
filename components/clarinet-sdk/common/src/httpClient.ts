import request, { HttpVerb, Options as HttpOptions } from "sync-request";

const encoder = new TextEncoder();

export function httpClient(method: HttpVerb, path: string, options?: HttpOptions): Uint8Array {
  console.log("-".repeat(20));
  console.log("httpClient", method, path, options);
  const response = request(method, path, options);
  if (typeof response.body === "string") {
    return encoder.encode(response.body);
  }
  console.log("httpClient response", response.statusCode);
  console.log("-".repeat(20));
  return response.body;
}
