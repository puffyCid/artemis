const { core } = globalThis.Deno;
class Request {
    send = (url, protocol, headers, body, body_type, follow_redirects) => {
        return core.ops.js_request(url, protocol, headers, body, body_type, follow_redirects);
    };
}
export const requst = new Request();
