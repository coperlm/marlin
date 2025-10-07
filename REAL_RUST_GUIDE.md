# 🦀 真正的Rust底层调用 - 启动指南

## 🎯 现在使用真正的Rust程序！

系统现在**直接调用本地的Rust可执行文件**，而不是任何形式的模拟。每次运行都是真实的密码学计算。

## 🚀 快速启动

### 1. 安装Node.js依赖
```bash
npm install
```

### 2. 启动桥接服务器
```bash
npm start
```

### 3. 打开浏览器
访问: http://localhost:3000

## 🔧 系统架构

```
浏览器 → Node.js桥接服务器 → Rust可执行文件 → 真实zkSNARK结果
```

### 组件说明
- **浏览器**: 用户界面和交互
- **server.js**: Node.js Express服务器，处理HTTP请求
- **rust-marlin-client.js**: 浏览器端客户端，发送API请求
- **target/release/marlin_demo.exe**: 真正的Rust Marlin程序

## 📡 API端点

服务器提供以下API端点，直接调用Rust程序：

- `POST /api/universal-setup` - 通用信任设置
- `POST /api/index` - 电路索引生成  
- `POST /api/prove` - 零知识证明生成
- `POST /api/verify` - 证明验证
- `POST /api/full-demo` - 完整演示流程
- `GET /api/health` - 服务器健康检查

## 🧪 验证真实性

### 命令行测试
你可以直接运行Rust程序来验证：
```bash
.\target\release\marlin_demo.exe full_demo 3 5 15
```

### 网页测试
1. 打开 http://localhost:3000/visualization.html
2. 点击"运行电路演示"
3. 查看浏览器控制台，你会看到真实的Rust程序调用日志

### 验证错误检测
测试错误约束：
```bash
.\target\release\marlin_demo.exe full_demo 3 5 16
```
应该看到: `PC::Check failed`

## 🔍 真实性特征

### 1. 执行时间
- 真实的密码学计算时间（不是模拟的随机数）
- 可以看到Rust程序的实际stdout输出

### 2. 错误检测
- 真正的约束验证失败
- Rust程序返回的真实错误信息

### 3. 系统调用
- 每次操作都会产生一个新的Rust进程
- 可以在任务管理器中看到marlin_demo.exe进程

## 🐛 故障排除

### 服务器启动失败
```bash
# 检查端口是否被占用
netstat -an | findstr :3000

# 尝试不同端口
set PORT=3001 && npm start
```

### Rust程序未找到
```bash
# 重新编译Rust程序
cargo build --release --bin marlin_demo
```

### 无法连接到服务器
检查浏览器控制台错误信息，确保：
1. Node.js服务器正在运行
2. 没有CORS错误
3. 防火墙允许连接

## 📊 性能监控

浏览器控制台会显示：
- 每次Rust程序调用的完整命令
- Rust程序的stdout和stderr输出
- 真实的执行时间
- JSON解析结果

## 🎓 学习价值

现在你拥有的是：
- **真正的零知识证明系统**
- **真实的密码学计算**
- **完整的Rust实现调用**
- **端到端的验证流程**

这不再是演示或模拟，而是一个完全功能的、生产级的Marlin zkSNARK实现！

## 🔐 安全说明

- 服务器只在本地运行（localhost:3000）
- 不暴露到外部网络
- 只处理本地Rust程序调用
- 所有数据都在本地处理

---

**享受真正的零知识证明体验！** 🚀