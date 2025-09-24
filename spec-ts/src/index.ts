import fs from 'fs';
import path from 'path';
import { 
  ToolSpecSchema, 
  PlanSchema, 
  EvidenceSchema, 
  MemorySchema, 
  TraceSchema,
  validateToolSpec,
  validatePlan,
  validateEvidence,
  validateMemory,
  validateTrace,
  type ToolSpec,
  type Plan,
  type Evidence,
  type Memory,
  type Trace
} from './types';

// Schema loaders
export const loadToolSpecSchema = (): object => {
  const schemaPath = path.join(__dirname, '../../schemas/ToolSpec.schema.json');
  return JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
};

export const loadPlanSchema = (): object => {
  const schemaPath = path.join(__dirname, '../../schemas/Plan.schema.json');
  return JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
};

export const loadEvidenceSchema = (): object => {
  const schemaPath = path.join(__dirname, '../../schemas/Evidence.schema.json');
  return JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
};

export const loadMemorySchema = (): object => {
  const schemaPath = path.join(__dirname, '../../schemas/Memory.schema.json');
  return JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
};

export const loadTraceSchema = (): object => {
  const schemaPath = path.join(__dirname, '../../schemas/Trace.schema.json');
  return JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
};

// Export everything
export {
  ToolSpecSchema,
  PlanSchema,
  EvidenceSchema,
  MemorySchema,
  TraceSchema,
  validateToolSpec,
  validatePlan,
  validateEvidence,
  validateMemory,
  validateTrace,
  type ToolSpec,
  type Plan,
  type Evidence,
  type Memory,
  type Trace
};