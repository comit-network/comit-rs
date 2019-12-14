// A helper file to configure chai with all the plugins we need
// This is to reduce noise in our actual test files

import chaiBn = require("bn-chai");
import { tv4, use } from "chai";
import chaiAsPromised from "chai-as-promised";
import chaiEach = require("chai-each");
import chaiHttp = require("chai-http");
import chaiJsonSchema = require("chai-json-schema");
import chaiSubset = require("chai-subset");
import { BigNumber } from "ethers/utils";
import * as sirenJsonSchema from "../siren.schema.json";

use(chaiAsPromised);
use(chaiHttp);
use(chaiSubset);
use(chaiEach);
use(chaiJsonSchema);
use(chaiBn(BigNumber));

tv4.addSchema("http://sirenspec.org/schema", sirenJsonSchema);
