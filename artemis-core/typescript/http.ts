//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

class Request {
    send = (url: string, protocol: string, headers: Record<string, string>, body: string) => {
        return core.ops.js_request(url, protocol, headers, body);
    }
}

export const requst = new Request();