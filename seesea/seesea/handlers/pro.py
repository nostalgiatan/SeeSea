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
SeeSea Pro API Handlers - Pro API处理器

提供Pro API的处理函数，包括搜索增强功能
"""

from typing import Dict, Any, List
import json
from datetime import datetime

from ..search import SearchClient
from ..Pro import ContentProcessor
from ..Pro.vector_utils import VectorUtils


async def handle_pro_search(req: Dict[str, Any]) -> Dict[str, Any]:
    """
    处理Pro API搜索请求

    执行增强搜索流程：
    1. 执行正常搜索获取基础结果
    2. 对每个结果URL进行内容处理
    3. 清洗内容并提取元数据
    4. 合并搜索结果与网页内容
    5. 存储到向量数据库
    6. 返回增强搜索结果

    Args:
        req: 请求上下文，包含方法、路径、查询参数和请求体

    Returns:
        Dict[str, Any]: 增强搜索结果
    """
    # 1. 解析请求参数
    query_params = req.get("query_params", {})
    request_body = req.get("body", {})

    # 获取查询关键词
    query = query_params.get("q") or query_params.get("query")
    if isinstance(request_body, str):
        request_body = json.loads(request_body)
    query = query or request_body.get("query") or request_body.get("q")

    if not query:
        return {"status": 400, "body": json.dumps({"error": "查询关键词不能为空"})}

    # 其他搜索参数
    page = query_params.get("page", 1)
    page_size = query_params.get("page_size", 10)

    # 2. 执行正常搜索
    search_client = SearchClient()
    search_response = search_client.search(query, page=page, page_size=page_size)

    # 3. 初始化Pro组件
    processor = ContentProcessor()

    # 初始化VectorUtils，启用批处理功能
    vector_utils = VectorUtils(
        batch_size=20, max_memory_mb=512  # 设置批处理大小  # 设置最大内存使用量
    )

    # 4. 处理每个搜索结果
    enhanced_results: List[Dict[str, Any]] = []
    for result in search_response.results:
        url = result.url
        title = result.title
        content = result.content

        # 这些属性在SearchResultItem中不存在，设置默认值
        published_date = None
        engine = None

        try:
            # 处理URL内容 - 调用异步方法，使用await等待完成
            processed_data = await processor.process_url(url, keywords=query)

            # 提取网页元数据
            web_metadata = processed_data.get("metadata", {})
            web_title = web_metadata.get("title", "")
            web_description = web_metadata.get("description", "")

            # 清洗后的数据块
            cleaned_data = processed_data.get("cleaned_data", [])

            # 5. 合并结果
            # 标题：搜索结果优先，网页标题补充
            final_title = title or web_title

            # 描述：网页描述优先，搜索结果内容补充
            final_description = web_description or content

            # 时间：使用网页元数据中的时间，如果没有则为None
            final_date = web_metadata.get("published_date", published_date)

            # 上下文：清洗后的数据块按照顺序拼接起来作为上下文
            context = ""
            for block in cleaned_data:
                context += block.get("content", "") + "\n\n"
            context = context.strip()
            
            # 如果上下文为空，使用描述作为备选
            if not context:
                context = final_description

            # 6. 构建增强结果
            enhanced_result = {
                "title": final_title,
                "url": url,
                "description": final_description,
                "published_date": final_date,
                "context": context,
                "source": engine,
                "score": result.score,  # 初始使用搜索结果分数
                "web_metadata": web_metadata,
                "processed_at": datetime.now().isoformat(),
            }

            # 7. 存储到向量数据库
            vector_utils.add_document(
                content=context,
                metadata={
                    "title": final_title,
                    "url": url,
                    "description": final_description,
                    "published_date": final_date,
                    "source": engine,
                    "query": query,
                    "processed_at": datetime.now().isoformat(),
                },
            )

            enhanced_results.append(enhanced_result)

        except Exception as e:
            # 如果处理失败，使用原始结果
            enhanced_results.append(
                {
                    "title": title,
                    "url": url,
                    "description": content,
                    "published_date": published_date,
                    "context": None,
                    "source": engine,
                    "score": result.score,
                    "error": str(e),
                    "processed_at": datetime.now().isoformat(),
                }
            )

    # 8. 强制处理所有剩余文档，确保所有文档都已存入向量数据库
    processed_count = vector_utils.flush()
    print(f"批处理结果：总共处理了 {processed_count} 个新文档或更新了 {processed_count} 个现有文档")

    # 9. 从向量数据库获取相关结果
    vector_results = vector_utils.search(query, limit=page_size)

    # 10. 融合搜索结果和向量结果，调整信任值
    # 创建URL到向量结果的映射，用于快速查找
    vector_result_map = {}
    for vector_result in vector_results:
        # 检查向量结果的结构，获取URL
        result_url = vector_result.get("url") or vector_result.get("metadata", {}).get("url")
        if result_url:
            vector_result_map[result_url] = vector_result

    # 调整增强结果的信任值，使用向量数据库提供的信任值
    for enhanced_result in enhanced_results:
        result_url = enhanced_result.get("url")
        if result_url and result_url in vector_result_map:
            vector_result = vector_result_map[result_url]
            # 尝试从不同位置获取score
            vector_score = vector_result.get("score")
            if vector_score is None:
                vector_score = vector_result.get("metadata", {}).get("score")
            if vector_score is not None:
                # 使用向量数据库的信任值替换原始搜索结果的信任值
                enhanced_result["score"] = vector_score
                # 记录调试信息
                print(
                    f"Updated score for {result_url} from {enhanced_result.get('score')} to {vector_score}"
                )
            else:
                # 记录调试信息
                print(f"No score found in vector result for {result_url}: {vector_result}")

    # 10. 构建最终响应
    final_response = {
        "status": 200,
        "body": json.dumps(
            {
                "query": query,
                "total_count": len(enhanced_results),
                "page": page,
                "page_size": page_size,
                "results": enhanced_results,
                "vector_results": vector_results,
                "engines_used": search_response.engines_used,
                "query_time_ms": search_response.query_time_ms,
                "cached": search_response.cached,
                "enhanced": True,
            }
        ),
    }

    return final_response


def register_pro_routes(api_server) -> None:
    """
    注册Pro API路由

    Args:
        api_server: API服务器实例
    """
    # 添加Pro搜索路由
    api_server.add_pro_route("/search", handle_pro_search, method="POST")
    api_server.add_pro_route("/search", handle_pro_search, method="GET")
