const { core } = globalThis.Deno;
class Request {
    send = (url, protocol, headers, body) => {
        return core.ops.js_request(url, protocol, headers, body);
    };
}
export const requst = new Request();
