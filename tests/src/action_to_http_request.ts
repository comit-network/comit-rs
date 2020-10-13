import { AxiosRequestConfig } from "axios";
import contentType from "content-type";
import URI from "urijs";
import { Action, Field } from "./cnd_client/siren";

export type FieldValueResolverFn = (
    field: Field
) => Promise<string | undefined>;

export default async function actionToHttpRequest(
    action: Action,
    resolver?: FieldValueResolverFn
): Promise<AxiosRequestConfig> {
    const fields = action.fields || [];
    const fieldValues = await resolveAllFieldValues(fields, resolver);

    const requestMethod = action.method ? action.method : "GET";

    if (requestMethod === "GET") {
        return Promise.resolve({
            url: action.href,
            method: action.method,
            params: fieldValues,
            paramsSerializer: (params: any) => {
                return URI.buildQuery(params);
            },
            data: {}, // Need to set this because of https://github.com/axios/axios/issues/86
        });
    } else {
        return Promise.resolve({
            url: action.href,
            method: action.method,
            data: fieldValues,
            transformRequest: [jsonRequestTransformer, failIfNotBuffer],
            headers: {
                "Content-Type": action.type,
            },
        });
    }
}

function jsonRequestTransformer(data: any, headers: any): any {
    const rawContentType = headers["Content-Type"];

    if (!rawContentType) {
        return data;
    }

    const parsedContentType = contentType.parse(rawContentType).type;

    if (parsedContentType !== "application/json") {
        return data; // pass on data to the next transformer
    }

    return Buffer.from(JSON.stringify(data), "utf-8");
}

function failIfNotBuffer(data: any, headers: any): any {
    if (data && !Buffer.isBuffer(data)) {
        throw new Error(
            `Failed to serialize data for content-type ${headers["Content-Type"]}`
        );
    }

    return data;
}

async function resolveAllFieldValues(
    fields: Field[],
    resolver?: FieldValueResolverFn
): Promise<any> {
    const data: any = {};

    if (!resolver) {
        return Promise.resolve(data);
    }

    for (const field of fields) {
        const value = await resolver(field);

        if (value) {
            data[field.name] = value;
        }
    }

    return Promise.resolve(data);
}
