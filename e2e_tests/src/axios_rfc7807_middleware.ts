import { AxiosError } from "axios";
import contentType from "content-type";

export class Problem extends Error {
    private static makeMessage({
        title,
        type,
        status,
        detail,
    }: ProblemMembers): string {
        const statusPart = status
            ? `Request failed with status code ${status}: `
            : `Request failed: `;
        const typePart =
            type && type !== "about:blank"
                ? ` See ${type} for more information.`
                : ``;
        const detailPart = detail ? ` ${detail}` : ``;

        return `${statusPart}${title}${detailPart}${typePart}`;
    }

    public readonly type: string;
    public readonly title: string;
    public readonly status?: number;
    public readonly detail?: string;
    public readonly instance?: string;

    constructor({ title, type, status, detail, instance }: ProblemMembers) {
        super(Problem.makeMessage({ title, type, status, detail, instance }));
        this.type = type || "about:blank";
        this.status = status;
        this.detail = detail;
        this.instance = instance;
        this.title = title;
    }
}

interface ProblemMembers {
    title: string;
    type?: string;
    status?: number;
    detail?: string;
    instance?: string;
}

export async function problemResponseInterceptor(
    error: AxiosError
): Promise<AxiosError | Problem> {
    const response = error.response;

    if (!response) {
        return Promise.reject(error);
    }

    const rawContentType = response.headers["content-type"];

    if (!rawContentType) {
        return Promise.reject(error);
    }

    const parsedContentType = contentType.parse(rawContentType).type;

    if (parsedContentType !== "application/problem+json") {
        return Promise.reject(error);
    }

    const responseBody = response.data;

    return Promise.reject(new Problem(responseBody));
}
