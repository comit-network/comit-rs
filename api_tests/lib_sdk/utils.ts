export async function sleep(ms: number) {
    return new Promise(res => {
        setTimeout(res, ms);
    });
}

export async function timeout<T>(ms: number, promise: Promise<T>): Promise<T> {
    // Create a promise that rejects in <ms> milliseconds
    const timeout = new Promise<T>((_, reject) => {
        const id = setTimeout(() => {
            clearTimeout(id);
            reject(new Error("timed out after " + ms + "ms"));
        }, ms);
    });

    // Returns a race between our timeout and the passed in promise
    return Promise.race([promise, timeout]);
}
