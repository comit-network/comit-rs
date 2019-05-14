// A helper file to configure chai with all the plugins we need
// This is to reduce noise in our actual test files

import { tv4, use } from "chai";
import * as sirenJsonSchema from "../siren.schema.json";
import chaiHttp = require("chai-http");
import chaiSubset = require("chai-subset");
import chaiEach = require("chai-each");
import chaiJsonSchema = require("chai-json-schema");

use(chaiHttp);
use(chaiSubset);
use(chaiEach);
use(chaiJsonSchema);

tv4.addSchema("http://sirenspec.org/schema", sirenJsonSchema);
