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
数据提供者模块

提供多种数据源的适配器实现。
"""

from .base import BaseProvider
from .akshare_provider import AKShareProvider, StockNameMapping, get_stock_mapping
from .cninfo_provider import CNInfoProvider

__all__ = [
    "BaseProvider",
    "AKShareProvider",
    "CNInfoProvider",
    "StockNameMapping",
    "get_stock_mapping",
]
