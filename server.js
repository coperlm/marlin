// 本地服务器，用于桥接浏览器和Rust Marlin程序
const express = require('express');
const { spawn } = require('child_process');
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = 3000;

// 中间件
app.use(cors());
app.use(express.json());
app.use(express.static('./')); // 静态文件服务

// Rust可执行文件路径
const RUST_EXECUTABLE = path.join(__dirname, 'target', 'release', 'marlin_demo.exe');
const CONSTRAINT_TESTER = path.join(__dirname, 'target', 'release', 'constraint_tester.exe');

// 执行Rust命令的辅助函数
function executeRustCommand(operation, args = [], executable = RUST_EXECUTABLE) {
    return new Promise((resolve, reject) => {
        console.log(`执行Rust命令: ${executable} ${operation} ${args.join(' ')}`);
        
        const child = spawn(executable, [operation, ...args], {
            cwd: __dirname,
            stdio: ['pipe', 'pipe', 'pipe']
        });

        let stdout = '';
        let stderr = '';

        child.stdout.on('data', (data) => {
            stdout += data.toString();
        });

        child.stderr.on('data', (data) => {
            stderr += data.toString();
        });

        child.on('close', (code) => {
            console.log(`Rust程序退出码: ${code}`);
            console.log(`标准输出: ${stdout}`);
            if (stderr) console.log(`标准错误: ${stderr}`);

            if (code === 0) {
                try {
                    // 尝试解析JSON输出
                    const lines = stdout.split('\n');
                    let jsonOutput = null;
                    
                    // 查找JSON输出
                    for (const line of lines) {
                        if (line.trim().startsWith('{')) {
                            try {
                                jsonOutput = JSON.parse(line.trim());
                                break;
                            } catch (e) {
                                // 继续寻找
                            }
                        }
                    }
                    
                    resolve({
                        success: true,
                        output: stdout,
                        jsonResult: jsonOutput,
                        exitCode: code
                    });
                } catch (error) {
                    resolve({
                        success: true,
                        output: stdout,
                        jsonResult: null,
                        exitCode: code,
                        parseError: error.message
                    });
                }
            } else {
                reject({
                    success: false,
                    error: stderr || stdout,
                    exitCode: code
                });
            }
        });

        child.on('error', (error) => {
            console.error('Rust程序执行错误:', error);
            reject({
                success: false,
                error: error.message
            });
        });
    });
}

// API端点：通用信任设置
app.post('/api/universal-setup', async (req, res) => {
    try {
        const result = await executeRustCommand('universal_setup');
        res.json(result);
    } catch (error) {
        res.status(500).json(error);
    }
});

// API端点：电路索引
app.post('/api/index', async (req, res) => {
    try {
        const result = await executeRustCommand('index');
        res.json(result);
    } catch (error) {
        res.status(500).json(error);
    }
});

// API端点：证明生成
app.post('/api/prove', async (req, res) => {
    try {
        const { a = 3, b = 5, c = 15 } = req.body;
        
        const result = await executeRustCommand('prove', [
            a.toString(),
            b.toString(),
            c.toString()
        ]);
        
        res.json(result);
    } catch (error) {
        res.status(500).json(error);
    }
});

// API端点：证明验证
app.post('/api/verify', async (req, res) => {
    try {
        const { a = 3, b = 5, c = 15 } = req.body;
        
        const result = await executeRustCommand('verify', [
            a.toString(),
            b.toString(),
            c.toString()
        ]);
        
        res.json(result);
    } catch (error) {
        res.status(500).json(error);
    }
});

// API端点：完整演示 - 使用新的约束测试器
app.post('/api/full-demo', async (req, res) => {
    try {
        const { a = 3, b = 5, c = 15 } = req.body;
        
        const result = await executeRustCommand('test_constraint', [
            a.toString(),
            b.toString(),
            c.toString()
        ], CONSTRAINT_TESTER);
        
        res.json(result);
    } catch (error) {
        res.status(500).json(error);
    }
});

// API端点：约束验证
app.post('/api/verify-constraint', async (req, res) => {
    try {
        const { a = 3, b = 5, c = 15 } = req.body;
        
        const result = await executeRustCommand('verify_constraint', [
            a.toString(),
            b.toString(),
            c.toString()
        ], CONSTRAINT_TESTER);
        
        res.json(result);
    } catch (error) {
        res.status(500).json(error);
    }
});

// API端点：健康检查
app.get('/api/health', (req, res) => {
    res.json({ 
        status: 'ok', 
        message: 'Marlin Rust Bridge Server is running',
        executable: RUST_EXECUTABLE,
        constraint_tester: CONSTRAINT_TESTER,
        timestamp: new Date().toISOString()
    });
});

// 启动服务器
app.listen(PORT, () => {
    console.log(`🦀 Marlin Rust Bridge Server 正在运行于端口 ${PORT}`);
    console.log(`🔗 可用的API端点:`);
    console.log('  POST /api/universal-setup');
    console.log('  POST /api/index');
    console.log('  POST /api/prove');
    console.log('  POST /api/verify');
    console.log('  POST /api/full-demo (使用新的约束测试器)');
    console.log('  POST /api/verify-constraint (快速约束验证)');
    console.log('  GET  /api/health');
    console.log(`🌐 访问 http://localhost:${PORT} 进行测试`);
    console.log(`📊 现在所有约束测试都是真实的Rust zkSNARK验证！`);
});

// 优雅关闭
process.on('SIGTERM', () => {
    console.log('正在关闭服务器...');
    process.exit(0);
});

process.on('SIGINT', () => {
    console.log('正在关闭服务器...');
    process.exit(0);
});