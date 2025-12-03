# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
