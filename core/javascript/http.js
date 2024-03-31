const { core } = globalThis.Deno;
class Request {
    send = (request, body) => {
        return core.ops.js_request(request, body);
    };
}
export const requst = new Request();
