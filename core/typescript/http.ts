//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

class Request {
    /**
     * Send HTTP requests
     * @param url URL to send request to
     * @param protocol HTTP method to use. Only GET or POST supported
     * @param headers Optional Headers to use
     * @param body Optional body to provide
     * @returns HTTP Response data
     */
    send = (url: string, protocol: string, headers: Record<string, string>, body: ArrayBuffer) => {
        return core.ops.js_request(url, protocol, headers, body);
    };
}

export const requst = new Request();