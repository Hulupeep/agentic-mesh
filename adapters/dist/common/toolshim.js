"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createToolServer = createToolServer;
/**
 * HTTP ToolSpec shim - uniform interface for all tools
 */
const express_1 = __importDefault(require("express"));
function createToolServer(tools, port) {
    const app = (0, express_1.default)();
    app.use(express_1.default.json());
    // Endpoint to get all tool specs
    app.get('/specs', (req, res) => {
        const specs = Object.keys(tools).map(name => tools[name].spec);
        res.json(specs);
    });
    // Endpoint to get a specific tool spec
    app.get('/spec/:name', (req, res) => {
        const toolName = req.params.name;
        const tool = tools[toolName];
        if (!tool) {
            res.status(404).json({ error: `Tool ${toolName} not found` });
            return;
        }
        res.json(tool.spec);
    });
    // Endpoint to invoke a tool
    app.post('/invoke/:name', async (req, res) => {
        const toolName = req.params.name;
        const tool = tools[toolName];
        if (!tool) {
            res.status(404).json({ error: `Tool ${toolName} not found` });
            return;
        }
        try {
            const result = await tool.invoke(req.body.args || {});
            res.json(result);
        }
        catch (error) {
            res.status(500).json({ error: error.message || 'Tool invocation failed' });
        }
    });
    app.listen(port, () => {
        console.log(`Tool server running on port ${port}`);
        console.log(`Available tools: ${Object.keys(tools).join(', ')}`);
    });
}
//# sourceMappingURL=toolshim.js.map