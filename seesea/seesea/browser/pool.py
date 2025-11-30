# Copyright 2025 nostalgiatan
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
Browser instance pool for SeeSea

This module provides a high-performance browser instance pool to optimize
JavaScript rendering by reusing browser instances and contexts instead of
creating new ones for each request.

Key Features:
- Browser instance pooling with configurable size
- Context pooling for faster request handling
- Automatic resource management and cleanup
- Async support for concurrent operations
- Smart allocation and reuse strategies
- Metrics tracking for performance monitoring

Architecture:
1. BrowserPool: Manages a pool of browser instances
2. ContextPool: Manages a pool of browser contexts per browser instance
3. PagePool: Manages a pool of pages per context (future enhancement)

Performance Benefits:
- Reduced browser startup/shutdown overhead
- Faster request handling through context reuse
- Lower memory usage compared to per-request browsers
- Better scalability for concurrent requests
"""

