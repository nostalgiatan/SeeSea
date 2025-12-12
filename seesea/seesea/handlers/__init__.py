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
SeeSea API Handlers - 处理API请求的处理器函数

提供各种API端点的处理函数
"""

# 导出处理器函数
from .pro import (
    handle_pro_search,
    add_pro_routes,
    initialize_pro_handlers,
    cleanup_command_line,
)

__all__ = [
    "handle_pro_search",
    "add_pro_routes",
    "initialize_pro_handlers",
    "cleanup_command_line",
]
