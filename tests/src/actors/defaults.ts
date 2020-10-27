export function defaultExpiries() {
    const { alphaAbsoluteExpiry, betaAbsoluteExpiry } = nowExpiries();

    return {
        alphaAbsoluteExpiry: alphaAbsoluteExpiry + 240,
        betaAbsoluteExpiry: betaAbsoluteExpiry + 120,
    };
}

export function nowExpiries() {
    const alphaAbsoluteExpiry = Math.round(Date.now() / 1000);
    const betaAbsoluteExpiry = Math.round(Date.now() / 1000);

    return {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
    };
}
