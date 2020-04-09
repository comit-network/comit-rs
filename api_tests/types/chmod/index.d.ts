declare module "chmod" {
    // there is more to this API but this is what we need
    export default function chmod(
        file: string,
        options: {
            execute?: boolean;
        }
    );
}
