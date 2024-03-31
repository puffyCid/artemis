//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

class Request {
    /**
     * Send HTTP requests
     * @param request JSON string representing a HTTP request
     * @param body Optional body to provide
     * @returns HTTP Response data
     */
    send = (request: string, body: Uint8Array) => {
        return core.ops.js_request(request, body);
    };
}

export const requst = new Request();