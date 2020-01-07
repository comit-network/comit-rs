declare module "readline-promise" {
    interface ReadLine {
        questionAsync(value: string): Promise<string>;
    }

    function createInterface(options: any): ReadLine;
}
