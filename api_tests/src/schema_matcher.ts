import Ajv from "ajv";

function validate(schema: object, data: object) {
    const ajv = new Ajv({ schemaId: "id", logger: false });
    ajv.addMetaSchema(require("ajv/lib/refs/json-schema-draft-04.json"));
    const valid = ajv.validate(schema, data);
    const errorText =
        ajv.errorsText() && ajv.errorsText().toLowerCase() !== "no errors"
            ? ajv.errorsText()
            : "";

    return {
        errorText,
        valid: !!valid,
    };
}

function extendSchemaMatcher(): void {
    expect.extend({
        toMatchSchema(data: object, schema: object) {
            const schemaValid = validate(schema, data);

            const pass = schemaValid.valid;
            const errorText = schemaValid.errorText;

            if (pass) {
                return {
                    pass,
                    message: () => errorText,
                };
            }
            return {
                pass,
                message: () => `data does not match Schema ${errorText}`,
            };
        },
    });
}

extendSchemaMatcher();
