export function crashListener(
    pid: number,
    component: string,
    logFile: string
): (exitCode: number | null) => void {
    return (exitCode) => {
        if (exitCode && exitCode !== 0) {
            throw new Error(
                `${component} ${pid} exited with code ${exitCode}, check ${logFile} for details`
            );
        }
    };
}
