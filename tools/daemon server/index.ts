import { setupFastify } from "./app";

async function main() {
    const server = await setupFastify();
    const listen = process.env.LISTEN ?? '127.0.0.1';

    server.listen({ port: 8000, host: listen }, (err, address) => {
        if (err) {
            console.error(err);
            process.exit(1);
        }
        console.log(`Mock server listening at ${address}`);
    });
}

main();