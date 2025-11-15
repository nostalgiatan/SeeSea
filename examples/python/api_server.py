#!/usr/bin/env python3
"""
SeeSea Python SDK - API Server Example
API 服务器示例
"""

from seesea import ApiServer

def main():
    print("🌊 SeeSea Python SDK - API 服务器示例\n")
    
    # 创建并启动 API 服务器
    server = ApiServer(host="127.0.0.1", port=8080)
    
    print(f"📍 API 端点:")
    print(f"  GET/POST {server.address}/api/search")
    print(f"  GET      {server.address}/api/engines")
    print(f"  GET      {server.address}/api/stats")
    print(f"  GET      {server.address}/api/health")
    print(f"  GET      {server.address}/api/version")
    print()
    
    # 启动服务器（阻塞）
    server.start()

if __name__ == "__main__":
    main()
