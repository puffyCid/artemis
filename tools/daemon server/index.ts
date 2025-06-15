import { setupFastify } from "./app";

function main() {
    const server = setupFastify();

    server.listen({ port: 8000 }, (err, address) => {
        if (err) {
            console.error(err);
            process.exit(1);
        }
        console.log(`Mock server listening at ${address}`);
    });
}

main();