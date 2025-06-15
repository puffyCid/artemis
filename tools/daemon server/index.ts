import { setupFastify } from "./app";

async function main() {
    const server = await setupFastify();

    server.listen({ port: 8000 }, (err, address) => {
        if (err) {
            console.error(err);
            process.exit(1);
        }
        console.log(`Mock server listening at ${address}`);
    });
}

main();