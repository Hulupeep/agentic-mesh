"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.validateTrace = exports.validateMemory = exports.validateEvidence = exports.validatePlan = exports.validateToolSpec = exports.TraceSchema = exports.MemorySchema = exports.EvidenceSchema = exports.PlanSchema = exports.ToolSpecSchema = exports.loadTraceSchema = exports.loadMemorySchema = exports.loadEvidenceSchema = exports.loadPlanSchema = exports.loadToolSpecSchema = void 0;
const fs_1 = __importDefault(require("fs"));
const path_1 = __importDefault(require("path"));
const types_1 = require("./types");
Object.defineProperty(exports, "ToolSpecSchema", { enumerable: true, get: function () { return types_1.ToolSpecSchema; } });
Object.defineProperty(exports, "PlanSchema", { enumerable: true, get: function () { return types_1.PlanSchema; } });
Object.defineProperty(exports, "EvidenceSchema", { enumerable: true, get: function () { return types_1.EvidenceSchema; } });
Object.defineProperty(exports, "MemorySchema", { enumerable: true, get: function () { return types_1.MemorySchema; } });
Object.defineProperty(exports, "TraceSchema", { enumerable: true, get: function () { return types_1.TraceSchema; } });
Object.defineProperty(exports, "validateToolSpec", { enumerable: true, get: function () { return types_1.validateToolSpec; } });
Object.defineProperty(exports, "validatePlan", { enumerable: true, get: function () { return types_1.validatePlan; } });
Object.defineProperty(exports, "validateEvidence", { enumerable: true, get: function () { return types_1.validateEvidence; } });
Object.defineProperty(exports, "validateMemory", { enumerable: true, get: function () { return types_1.validateMemory; } });
Object.defineProperty(exports, "validateTrace", { enumerable: true, get: function () { return types_1.validateTrace; } });
// Schema loaders
const loadToolSpecSchema = () => {
    const schemaPath = path_1.default.join(__dirname, '../../schemas/ToolSpec.schema.json');
    return JSON.parse(fs_1.default.readFileSync(schemaPath, 'utf8'));
};
exports.loadToolSpecSchema = loadToolSpecSchema;
const loadPlanSchema = () => {
    const schemaPath = path_1.default.join(__dirname, '../../schemas/Plan.schema.json');
    return JSON.parse(fs_1.default.readFileSync(schemaPath, 'utf8'));
};
exports.loadPlanSchema = loadPlanSchema;
const loadEvidenceSchema = () => {
    const schemaPath = path_1.default.join(__dirname, '../../schemas/Evidence.schema.json');
    return JSON.parse(fs_1.default.readFileSync(schemaPath, 'utf8'));
};
exports.loadEvidenceSchema = loadEvidenceSchema;
const loadMemorySchema = () => {
    const schemaPath = path_1.default.join(__dirname, '../../schemas/Memory.schema.json');
    return JSON.parse(fs_1.default.readFileSync(schemaPath, 'utf8'));
};
exports.loadMemorySchema = loadMemorySchema;
const loadTraceSchema = () => {
    const schemaPath = path_1.default.join(__dirname, '../../schemas/Trace.schema.json');
    return JSON.parse(fs_1.default.readFileSync(schemaPath, 'utf8'));
};
exports.loadTraceSchema = loadTraceSchema;
//# sourceMappingURL=index.js.map