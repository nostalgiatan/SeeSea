#!/usr/bin/env python3
"""
Demo example showing how to use the enhanced SeeSea API Server

This demonstrates:
1. Creating API server with different network modes
2. Using helper methods to inspect available endpoints
3. Starting the server (commented out to avoid blocking)
"""

from seesea.api import ApiServer

def main():
    print("=" * 60)
    print("SeeSea API Server - Complete Usage Demo")
    print("=" * 60)
    
    # Example 1: Internal mode (for local development)
    print("\n📖 Example 1: Creating an internal mode server")
    print("   (No security, suitable for local development)")
    server_internal = ApiServer(
        host="127.0.0.1",
        port=8080,
        network_mode="internal"
    )
    print(f"   Created: {server_internal}")
    print(f"   Access at: {server_internal.url}")
    
    # Show available endpoints
    print("\n   Available endpoints:")
    endpoints = server_internal.get_endpoints()
    for category, routes in endpoints.items():
        print(f"      {category.upper()}: {len(routes)} routes")
    
    # Example 2: External mode (for production)
    print("\n📖 Example 2: Creating an external mode server")
    print("   (With security features enabled)")
    server_external = ApiServer(
        host="0.0.0.0",
        port=8080,
        network_mode="external"
    )
    print(f"   Created: {server_external}")
    print(f"   Security features:")
    print(f"      - Rate limiting")
    print(f"      - Circuit breaker")
    print(f"      - IP filtering")
    print(f"      - JWT authentication")
    print(f"      - Magic link support")
    
    # Example 3: Dual mode (both internal and external)
    print("\n📖 Example 3: Creating a dual mode server")
    print("   (Runs both internal and external simultaneously)")
    server_dual = ApiServer(
        network_mode="dual"
    )
    print(f"   Created: {server_dual}")
    
    # Show detailed endpoint listing
    print("\n📖 Example 4: Detailed endpoint listing")
    server_internal.print_endpoints()
    
    # Example 5: Starting the server (commented to avoid blocking)
    print("\n📖 Example 5: Starting the server")
    print("   Uncomment one of the following to start:")
    print("   # server_internal.start()        # Default mode")
    print("   # server_internal.start_internal()  # Explicit internal")
    print("   # server_external.start_external()  # Explicit external")
    
    # Example API usage
    print("\n📖 Example 6: Using the API")
    print("   Once started, you can access endpoints like:")
    print(f"   - Search: curl {server_internal.url}/api/search?q=python")
    print(f"   - Health: curl {server_internal.url}/api/health")
    print(f"   - Stats:  curl {server_internal.url}/api/stats")
    print(f"   - Engines: curl {server_internal.url}/api/engines")
    
    print("\n" + "=" * 60)
    print("✅ Demo complete! Ready to use SeeSea API Server")
    print("=" * 60)

if __name__ == "__main__":
    main()
