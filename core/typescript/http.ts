//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

class Request {
    /**
     * Send HTTP requests
     * @param url URL to send request to
     * @param protocol HTTP method to use. Only GET or POST supported
     * @param headers Optional Headers to use
     * @param body Optional body to provide
     * @param body_type Type of body to submit. Either form or normal
     * @param follow_redirects Determine if artemis should follow redirect responses
     * @returns HTTP Response data
     */
    send = (url: string, protocol: string, headers: Record<string, string>, body: Uint8Array, body_type: string, follow_redirects: string) => {
        return core.ops.js_request(url, protocol, headers, body, body_type, follow_redirects);
    };
}

export const requst = new Request();