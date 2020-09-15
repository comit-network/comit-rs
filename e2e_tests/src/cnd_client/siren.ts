export type SubEntity = EmbeddedLinkSubEntity | EmbeddedRepresentationSubEntity;
export type RelValue =
    | string
    | (
          | "about"
          | "alternate"
          | "appendix"
          | "archives"
          | "author"
          | "blocked-by"
          | "bookmark"
          | "canonical"
          | "chapter"
          | "collection"
          | "contents"
          | "convertedFrom"
          | "copyright"
          | "create-form"
          | "current"
          | "derivedfrom"
          | "describedby"
          | "describes"
          | "disclosure"
          | "dns-prefetch"
          | "duplicate"
          | "edit"
          | "edit-form"
          | "edit-media"
          | "enclosure"
          | "first"
          | "glossary"
          | "help"
          | "hosts"
          | "hub"
          | "icon"
          | "index"
          | "item"
          | "last"
          | "latest-version"
          | "license"
          | "lrdd"
          | "memento"
          | "monitor"
          | "monitor-group"
          | "next"
          | "next-archive"
          | "nofollow"
          | "noreferrer"
          | "original"
          | "payment"
          | "pingback"
          | "preconnect"
          | "predecessor-version"
          | "prefetch"
          | "preload"
          | "prerender"
          | "prev"
          | "preview"
          | "previous"
          | "prev-archive"
          | "privacy-policy"
          | "profile"
          | "related"
          | "restconf"
          | "replies"
          | "search"
          | "section"
          | "self"
          | "service"
          | "start"
          | "stylesheet"
          | "subsection"
          | "successor-version"
          | "tag"
          | "terms-of-service"
          | "timegate"
          | "timemap"
          | "type"
          | "up"
          | "version-history"
          | "via"
          | "webmention"
          | "working-copy"
          | "working-copy-of"
      );
/**
 * Defines media type of the linked resource, per Web Linking (RFC5988). For the syntax, see RFC2045 (section 5.1), RFC4288 (section 4.2), RFC6838 (section 4.2)
 */
export type MediaType = string;
export type EmbeddedRepresentationSubEntity = Entity & {
    /**
     * Defines the relationship of the sub-entity to its parent, per Web Linking (RFC5899).
     */
    rel: [RelValue, ...RelValue[]];
    [k: string]: any;
};

/**
 * An Entity is a URI-addressable resource that has properties and actions associated with it. It may contain sub-entities and navigational links.
 */
export interface Entity {
    /**
     * Describes the nature of an entity's content based on the current representation. Possible values are implementation-dependent and should be documented.
     */
    class?: string[];
    /**
     * Descriptive text about the entity.
     */
    title?: string;
    /**
     * A set of key-value pairs that describe the state of an entity.
     */
    properties?: {
        [k: string]: any;
    };
    /**
     * A collection of related sub-entities. If a sub-entity contains an href value, it should be treated as an embedded link. Clients may choose to optimistically load embedded links. If no href value exists, the sub-entity is an embedded entity representation that contains all the characteristics of a typical entity. One difference is that a sub-entity MUST contain a rel attribute to describe its relationship to the parent entity.
     */
    entities?: SubEntity[];
    /**
     * A collection of actions; actions show available behaviors an entity exposes.
     */
    actions?: Action[];
    /**
     * A collection of items that describe navigational links, distinct from entity relationships. Link items should contain a `rel` attribute to describe the relationship and an `href` attribute to point to the target URI. Entities should include a link `rel` to `self`.
     */
    links?: Link[];
    [k: string]: any;
}
export interface EmbeddedLinkSubEntity {
    /**
     * Describes the nature of an entity's content based on the current representation. Possible values are implementation-dependent and should be documented.
     */
    class?: string[];
    /**
     * Defines the relationship of the sub-entity to its parent, per Web Linking (RFC5899).
     */
    rel: [RelValue, ...RelValue[]];
    /**
     * The URI of the linked sub-entity.
     */
    href: string;
    type?: MediaType;
    /**
     * Descriptive text about the entity.
     */
    title?: string;
    [k: string]: any;
}
/**
 * Actions show available behaviors an entity exposes.
 */
export interface Action {
    /**
     * Describes the nature of an action based on the current representation. Possible values are implementation-dependent and should be documented.
     */
    class?: string[];
    /**
     * A string that identifies the action to be performed. Action names MUST be unique within the set of actions for an entity. The behaviour of clients when parsing a Siren document that violates this constraint is undefined.
     */
    name: string;
    /**
     * An enumerated attribute mapping to a protocol method. For HTTP, these values may be GET, PUT, POST, DELETE, or PATCH. As new methods are introduced, this list can be extended. If this attribute is omitted, GET should be assumed.
     */
    method?: "DELETE" | "GET" | "PATCH" | "POST" | "PUT";
    /**
     * The URI of the action.
     */
    href: string;
    /**
     * Descriptive text about the action.
     */
    title?: string;
    /**
     * The encoding type for the request. When omitted and the fields attribute exists, the default value is `application/x-www-form-urlencoded`.
     */
    type?: string;
    /**
     * A collection of fields.
     */
    fields?: Field[];
    [k: string]: any;
}
/**
 * Fields represent controls inside of actions.
 */
export interface Field {
    /**
     * A name describing the control. Field names MUST be unique within the set of fields for an action. The behaviour of clients when parsing a Siren document that violates this constraint is undefined.
     */
    name: string;
    /**
     * The input type of the field. This is a subset of the input types specified by HTML5.
     */
    type?:
        | "hidden"
        | "text"
        | "search"
        | "tel"
        | "url"
        | "email"
        | "password"
        | "datetime"
        | "date"
        | "month"
        | "week"
        | "time"
        | "datetime-local"
        | "number"
        | "range"
        | "color"
        | "checkbox"
        | "radio"
        | "file";
    /**
     * Textual annotation of a field. Clients may use this as a label.
     */
    title?: string;
    /**
     * A value assigned to the field.  May be a scalar value or a list of value objects.
     */
    value?: (string | number) | FieldValueObject[];
    [k: string]: any;
}
/**
 * Value objects represent multiple selectable field values. Use in conjunction with field `"type" = "radio"` and `"type" = "checkbox"` to express that zero, one or many out of several possible values may be sent back to the server.
 */
export interface FieldValueObject {
    /**
     * Textual description of a field value.
     */
    title?: string;
    /**
     * Possible value for the field.
     */
    value: string | number;
    /**
     * A value object with a `"selected" = true` attribute indicates that this value should be considered preselected by the client. When missing, the default value is `false`.
     */
    selected?: boolean;
    [k: string]: any;
}
/**
 * Links represent navigational transitions.
 */
export interface Link {
    /**
     * Describes aspects of the link based on the current representation. Possible values are implementation-dependent and should be documented.
     */
    class?: string[];
    /**
     * Text describing the nature of a link.
     */
    title?: string;
    /**
     * Defines the relationship of the link to its entity, per Web Linking (RFC5988).
     */
    rel: RelValue[];
    /**
     * The URI of the linked resource.
     */
    href: string;
    type?: MediaType;
    [k: string]: any;
}
