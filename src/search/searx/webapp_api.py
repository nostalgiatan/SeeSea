#!/usr/bin/env python
# SPDX-License-Identifier: AGPL-3.0-or-later
"""WebApp API-only version - removes UI components"""
# pylint: disable=use-dict-literal

import json
import os
import sys
import base64

from timeit import default_timer
from io import StringIO
import typing

import urllib
import urllib.parse
from urllib.parse import urlencode, urlparse

import warnings
import httpx

import flask

from flask import (
    Flask,
    make_response,
    redirect,
    send_from_directory,
)
from flask.wrappers import Response
from flask.json import jsonify

import searx
from searx.extended_types import sxng_request
from searx import (
    logger,
    get_setting,
    settings,
)

from searx import limiter
from searx.botdetection import link_token, ProxyFix

from searx.settings_defaults import OUTPUT_FORMATS
from searx.settings_loader import DEFAULT_SETTINGS_FILE
from searx.exceptions import SearxParameterException
from searx.engines import (
    DEFAULT_CATEGORY,
    categories,
    engines,
    engine_shortcuts,
)

from searx import webutils
from searx.webadapter import (
    get_search_query_from_webapp,
    get_selected_categories,
    parse_lang,
)
from searx.utils import gen_useragent, dict_subset
from searx.version import VERSION_STRING, GIT_URL, GIT_BRANCH
from searx.query import RawTextQuery
from searx.plugins.oa_doi_rewrite import get_doi_resolver
from searx.preferences import (
    Preferences,
    ClientPref,
    ValidationException,
)
import searx.answerers
import searx.plugins


from searx.metrics import get_engines_stats, get_engine_errors, get_reliabilities, histogram, counter, openmetrics
from searx.flaskfix import patch_application

import searx.search
from searx.network import stream as http_stream, set_context_network_name
from searx.search.checker import get_result as checker_get_result


logger = logger.getChild('webapp_api')

warnings.simplefilter("always")

# Flask app (API-only)
app = Flask(__name__, static_folder=None, template_folder=None)
app.secret_key = settings['server']['secret_key']

STATS_SORT_PARAMETERS = {
    'name': (False, 'name', ''),
    'score': (True, 'score_per_result', 0),
    'result_count': (True, 'result_count', 0),
    'time': (False, 'total', 0),
    'reliability': (False, 'reliability', 100),
}


@app.before_request
def pre_request():
    sxng_request.start_time = default_timer()  # pylint: disable=assigning-non-slot
    sxng_request.render_time = 0  # pylint: disable=assigning-non-slot
    sxng_request.timings = []  # pylint: disable=assigning-non-slot
    sxng_request.errors = []  # pylint: disable=assigning-non-slot

    client_pref = ClientPref.from_http_request(sxng_request)
    # For API mode, use minimal preferences
    preferences = Preferences(['simple'], [], engines, searx.plugins.STORAGE, client_pref)

    user_agent = sxng_request.headers.get('User-Agent', '').lower()
    if 'webkit' in user_agent and 'android' in user_agent:
        preferences.key_value_settings['method'].value = 'GET'
    sxng_request.preferences = preferences  # pylint: disable=assigning-non-slot

    try:
        preferences.parse_dict(sxng_request.cookies)
    except Exception as e:  # pylint: disable=broad-except
        logger.exception(e, exc_info=True)
        sxng_request.errors.append('Invalid settings')

    # merge GET, POST vars
    sxng_request.form = dict(sxng_request.form.items())  # type: ignore
    for k, v in sxng_request.args.items():
        if k not in sxng_request.form:
            sxng_request.form[k] = v

    if sxng_request.form.get('preferences'):
        preferences.parse_encoded_data(sxng_request.form['preferences'])
    else:
        try:
            preferences.parse_dict(sxng_request.form)
        except Exception as e:  # pylint: disable=broad-except
            logger.exception(e, exc_info=True)
            sxng_request.errors.append('Invalid settings')

    # language is defined neither in settings nor in preferences
    if not preferences.get_value("language"):
        language = settings['search']['default_lang']
        preferences.parse_dict({"language": language})
        logger.debug('set language %s (default)', preferences.get_value("language"))

    # request.user_plugins
    sxng_request.user_plugins = []  # pylint: disable=assigning-non-slot
    allowed_plugins = preferences.plugins.get_enabled()
    disabled_plugins = preferences.plugins.get_disabled()
    for plugin in searx.plugins.STORAGE:
        if (plugin.id not in disabled_plugins) or plugin.id in allowed_plugins:
            sxng_request.user_plugins.append(plugin.id)


@app.after_request
def add_default_headers(response: flask.Response):
    # set default http headers
    for header, value in settings['server']['default_http_headers'].items():
        if header in response.headers:
            continue
        response.headers[header] = value
    return response


@app.after_request
def post_request(response: flask.Response):
    total_time = default_timer() - sxng_request.start_time
    timings_all = [
        'total;dur=' + str(round(total_time * 1000, 3)),
        'render;dur=' + str(round(sxng_request.render_time * 1000, 3)),
    ]
    if len(sxng_request.timings) > 0:
        timings = sorted(sxng_request.timings, key=lambda t: t.total)
        timings_total = [
            'total_' + str(i) + '_' + t.engine + ';dur=' + str(round(t.total * 1000, 3)) for i, t in enumerate(timings)
        ]
        timings_load = [
            'load_' + str(i) + '_' + t.engine + ';dur=' + str(round(t.load * 1000, 3))
            for i, t in enumerate(timings)
            if t.load
        ]
        timings_all = timings_all + timings_total + timings_load
    response.headers.add('Server-Timing', ', '.join(timings_all))
    return response


def api_error(output_format: str, error_message: str, status_code: int = 400):
    if output_format == 'json':
        return Response(json.dumps({'error': error_message}), mimetype='application/json', status=status_code)
    if output_format == 'csv':
        response = Response('', mimetype='application/csv', status=status_code)
        return response
    if output_format == 'rss':
        response_rss = '<?xml version="1.0" encoding="UTF-8"?><rss version="2.0"></rss>'
        return Response(response_rss, mimetype='text/xml', status=status_code)

    # Default to JSON for API mode
    return Response(json.dumps({'error': error_message}), mimetype='application/json', status=status_code)


def get_api_json_response(search_query, result_container):
    """API-safe version of get_json_response that doesn't require Flask-Babel."""
    from searx.search.models import UnresponsiveEngine

    # API-safe error mapping (without translations)
    api_exception_map = {
        None: 'unexpected crash',
        'timeout': 'timeout',
        'asyncio.TimeoutError': 'timeout',
        'httpx.TimeoutException': 'timeout',
        'httpx.ConnectTimeout': 'timeout',
        'httpx.ReadTimeout': 'timeout',
        'httpx.WriteTimeout': 'timeout',
        'httpx.HTTPStatusError': 'HTTP error',
        'httpx.ConnectError': 'HTTP connection error',
        'httpx.RemoteProtocolError': 'HTTP protocol error',
        'httpx.LocalProtocolError': 'HTTP protocol error',
        'httpx.ProtocolError': 'HTTP protocol error',
        'httpx.ReadError': 'network error',
        'httpx.WriteError': 'network error',
        'httpx.ProxyError': 'proxy error',
        'searx.exceptions.SearxEngineCaptchaException': 'CAPTCHA',
        'searx.exceptions.SearxEngineTooManyRequestsException': 'too many requests',
        'searx.exceptions.SearxEngineAccessDeniedException': 'access denied',
        'searx.exceptions.SearxEngineAPIException': 'server API error',
        'searx.exceptions.SearxEngineXPathException': 'parsing error',
        'KeyError': 'parsing error',
        'json.decoder.JSONDecodeError': 'parsing error',
        'lxml.etree.ParserError': 'parsing error',
        'ssl.SSLCertVerificationError': 'SSL error: certificate validation has failed',
        'ssl.CertificateError': 'SSL error: certificate validation has failed',
    }

    def get_api_errors(unresponsive_engines):
        api_errors = []
        for unresponsive_engine in unresponsive_engines:
            error_user_text = api_exception_map.get(unresponsive_engine.error_type, api_exception_map[None])
            if unresponsive_engine.suspended:
                error_user_text = 'Suspended: ' + error_user_text
            api_errors.append({
                'engine': unresponsive_engine.engine,
                'error': error_user_text,
                'parameter': unresponsive_engine.parameter
            })
        return api_errors

    results = result_container.get_ordered_results()

    # Create API-safe results (no HTML template processing)
    api_results = []
    for result in results:
        api_result = {
            'title': result.get('title', ''),
            'url': result.get('url', ''),
            'content': result.get('content', '')
        }

        # Add optional fields if they exist
        if 'img_src' in result:
            api_result['img_src'] = result['img_src']
        if 'thumbnail' in result:
            api_result['thumbnail'] = result['thumbnail']
        if 'template' in result:
            api_result['template'] = result['template']
        if 'engine' in result:
            api_result['engine'] = result['engine']
        if 'parsed_url' in result:
            api_result['parsed_url'] = result['parsed_url']
        if 'score' in result:
            api_result['score'] = result['score']

        api_results.append(api_result)

    return json.dumps({
        'query': search_query.query,
        'number_of_results': len(api_results),
        'results': api_results,
        'answers': list(result_container.answers),
        'corrections': list(result_container.corrections),
        'infoboxes': list(result_container.infoboxes),
        'suggestions': list(result_container.suggestions),
        'unresponsive_engines': get_api_errors(result_container.unresponsive_engines),
    })


@app.route('/healthz', methods=['GET'])
def health():
    return Response('OK', mimetype='text/plain')


@app.route('/client<token>.css', methods=['GET', 'POST'])
def client_token(token=None):
    link_token.ping(sxng_request, token)
    return Response('', mimetype='text/css', headers={"Cache-Control": "no-store, max-age=0"})


@app.route('/search', methods=['GET', 'POST'])
def search():
    """Search query in q and return results.

    Supported outputs: json, csv, rss.
    """
    # pylint: disable=too-many-locals, too-many-return-statements, too-many-branches
    # pylint: disable=too-many-statements

    # output_format - default to json for API mode
    output_format = sxng_request.form.get('format', 'json')
    if output_format not in OUTPUT_FORMATS:
        output_format = 'json'

    if output_format not in settings['search']['formats']:
        flask.abort(403)

    # check if there is query (not None and not an empty string)
    if not sxng_request.form.get('q'):
        return api_error(output_format, 'No query'), 400

    # search
    search_query = None
    raw_text_query = None
    result_container = None
    try:
        search_query, raw_text_query, _, _, selected_locale = get_search_query_from_webapp(
            sxng_request.preferences, sxng_request.form
        )
        search_obj = searx.search.SearchWithPlugins(search_query, sxng_request, sxng_request.user_plugins)
        result_container = search_obj.search()

    except SearxParameterException as e:
        logger.exception('search error: SearxParameterException')
        return api_error(output_format, e.message), 400
    except Exception as e:  # pylint: disable=broad-except
        logger.exception(e, exc_info=True)
        return api_error(output_format, 'search error'), 500

    # 1. check if the result is a redirect for an external bang
    if result_container.redirect_url:
        return redirect(result_container.redirect_url)

    # 2. add Server-Timing header for measuring performance characteristics
    sxng_request.timings = result_container.get_timings()  # pylint: disable=assigning-non-slot

    # 3. formats without a template
    if output_format == 'json':
        response = get_api_json_response(search_query, result_container)
        return Response(response, mimetype='application/json')

    if output_format == 'csv':
        csv = webutils.CSVWriter(StringIO())
        webutils.write_csv_response(csv, result_container)
        csv.stream.seek(0)

        response = Response(csv.stream.read(), mimetype='application/csv')
        cont_disp = 'attachment;Filename=searx_-_{0}.csv'.format(search_query.query)
        response.headers.add('Content-Disposition', cont_disp)
        return response

    # 4. RSS format
    if output_format == 'rss':
        results = result_container.get_ordered_results()

        # Simple RSS XML generation
        rss_xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>SearXNG Search Results</title>
    <description>Search results for: {sxng_request.form.get('q', '')}</description>
    <link>{settings['server'].get('base_url', '')}</link>
    <generator>SearXNG {VERSION_STRING}</generator>'''

        for result in results:
            rss_xml += f'''
    <item>
      <title>{result.get('title', '')}</title>
      <link>{result.get('url', '')}</link>
      <description>{result.get('content', '')}</description>
    </item>'''

        rss_xml += '''
  </channel>
</rss>'''
        return Response(rss_xml, mimetype='text/xml')

    # Default to JSON
    response = webutils.get_json_response(search_query, result_container)
    return Response(response, mimetype='application/json')


@app.route('/autocompleter', methods=['GET', 'POST'])
def autocompleter():
    """Return autocompleter results"""

    # run autocompleter
    results = []

    # set blocked engines
    disabled_engines = sxng_request.preferences.engines.get_disabled()

    # parse query
    raw_text_query = RawTextQuery(sxng_request.form.get('q', ''), disabled_engines)
    sug_prefix = raw_text_query.getQuery()

    for obj in searx.answerers.STORAGE.ask(sug_prefix):
        if hasattr(obj, 'answer'):
            results.append(obj.answer)

    # normal autocompletion results only appear if no inner results returned
    # and there is a query part
    if len(raw_text_query.autocomplete_list) == 0 and len(sug_prefix) > 0:

        # get SearXNG's locale and autocomplete backend from cookie
        sxng_locale = sxng_request.preferences.get_value('language')
        backend_name = sxng_request.preferences.get_value('autocomplete')

        from searx.autocomplete import search_autocomplete
        for result in search_autocomplete(backend_name, sug_prefix, sxng_locale):
            # attention: this loop will change raw_text_query object and this is
            # the reason why the sug_prefix was stored before (see above)
            if result != sug_prefix:
                results.append(raw_text_query.changeQuery(result).getFullQuery())

    if len(raw_text_query.autocomplete_list) > 0:
        for autocomplete_text in raw_text_query.autocomplete_list:
            results.append(raw_text_query.get_autocomplete_full_query(autocomplete_text))

    if sxng_request.headers.get('X-Requested-With') == 'XMLHttpRequest':
        # the suggestion request comes from the searx search form
        suggestions = json.dumps(results)
        mimetype = 'application/json'
    else:
        # the suggestion request comes from browser's URL bar
        suggestions = json.dumps([sug_prefix, results])
        mimetype = 'application/x-suggestions+json'

    suggestions = json.dumps(results)  # Simplified for API mode
    return Response(suggestions, mimetype='application/json')


@app.route('/image_proxy', methods=['GET'])
def image_proxy():
    # pylint: disable=too-many-return-statements, too-many-branches

    url = sxng_request.args.get('url')
    if not url:
        return '', 400

    if not is_hmac_of(settings['server']['secret_key'], url.encode(), sxng_request.args.get('h', '')):
        return '', 400

    maximum_size = 5 * 1024 * 1024
    forward_resp = False
    resp = None
    try:
        request_headers = {
            'User-Agent': gen_useragent(),
            'Accept': 'image/webp,*/*',
            'Sec-GPC': '1',
            'DNT': '1',
        }
        set_context_network_name('image_proxy')
        resp, stream = http_stream(method='GET', url=url, headers=request_headers, allow_redirects=True)
        content_length = resp.headers.get('Content-Length')
        if content_length and content_length.isdigit() and int(content_length) > maximum_size:
            return 'Max size', 400

        if resp.status_code != 200:
            logger.debug('image-proxy: wrong response code: %i', resp.status_code)
            if resp.status_code >= 400:
                return '', resp.status_code
            return '', 400

        if not resp.headers.get('Content-Type', '').startswith('image/') and not resp.headers.get(
            'Content-Type', ''
        ).startswith('binary/octet-stream'):
            logger.debug('image-proxy: wrong content-type: %s', resp.headers.get('Content-Type', ''))
            return '', 400

        forward_resp = True
    except httpx.HTTPError:
        logger.exception('HTTP error')
        return '', 400
    finally:
        if resp and not forward_resp:
            # the code is about to return an HTTP 400 error to the browser
            # we make sure to close the response between searxng and the HTTP server
            try:
                resp.close()
            except httpx.HTTPError:
                logger.exception('HTTP error on closing')

    def close_stream():
        nonlocal resp, stream
        try:
            if resp:
                resp.close()
            del resp
            del stream
        except httpx.HTTPError as e:
            logger.debug('Exception while closing response', e)

    try:
        headers = dict_subset(resp.headers, {'Content-Type', 'Content-Encoding', 'Content-Length', 'Length'})
        response = Response(stream, mimetype=resp.headers['Content-Type'], headers=headers, direct_passthrough=True)
        response.call_on_close(close_stream)
        return response
    except httpx.HTTPError:
        close_stream()
        return '', 400


@app.route('/engine_descriptions.json', methods=['GET'])
def engine_descriptions():
    from searx.data import ENGINE_DESCRIPTIONS
    from searx.locales import LOCALE_BEST_MATCH

    # Default to English for API mode
    sxng_ui_lang_tag = 'en'
    sxng_ui_lang_tag = LOCALE_BEST_MATCH.get(sxng_ui_lang_tag, sxng_ui_lang_tag)

    result = ENGINE_DESCRIPTIONS['en'].copy()
    if sxng_ui_lang_tag != 'en':
        for engine, description in ENGINE_DESCRIPTIONS.get(sxng_ui_lang_tag, {}).items():
            result[engine] = description
    for engine, description in result.items():
        if len(description) == 2 and description[1] == 'ref':
            ref_engine, ref_lang = description[0].split(':')
            description = ENGINE_DESCRIPTIONS[ref_lang][ref_engine]
        if isinstance(description, str):
            description = [description, 'wikipedia']
        result[engine] = description

    # overwrite by about:description (from settings)
    for engine_name, engine_mod in engines.items():
        descr = getattr(engine_mod, 'about', {}).get('description', None)
        if descr is not None:
            result[engine_name] = [descr, "SearXNG config"]

    return jsonify(result)


@app.route('/stats/errors', methods=['GET'])
def stats_errors():
    filtered_engines = dict(filter(lambda kv: sxng_request.preferences.validate_token(kv[1]), engines.items()))
    result = get_engine_errors(filtered_engines)
    return jsonify(result)


@app.route('/stats/checker', methods=['GET'])
def stats_checker():
    result = checker_get_result()
    return jsonify(result)


@app.route('/metrics')
def stats_open_metrics():
    password = settings['general'].get("open_metrics")

    if not (settings['general'].get("enable_metrics") and password):
        return Response('open metrics is disabled', status=404, mimetype='text/plain')

    if not sxng_request.authorization or sxng_request.authorization.password != password:
        return Response('access forbidden', status=401, mimetype='text/plain')

    filtered_engines = dict(filter(lambda kv: sxng_request.preferences.validate_token(kv[1]), engines.items()))

    checker_results = checker_get_result()
    checker_results = (
        checker_results['engines'] if checker_results['status'] == 'ok' and 'engines' in checker_results else {}
    )

    engine_stats = get_engines_stats(filtered_engines)
    engine_reliabilities = get_reliabilities(filtered_engines, checker_results)
    metrics_text = openmetrics(engine_stats, engine_reliabilities)

    return Response(metrics_text, mimetype='text/plain')


@app.route('/robots.txt', methods=['GET'])
def robots():
    return Response(
        """User-agent: *
Disallow: /stats
Disallow: /image_proxy
Disallow: /*?*q=*
""",
        mimetype='text/plain',
    )


@app.route('/opensearch.xml', methods=['GET'])
def opensearch():
    method = sxng_request.preferences.get_value('method')
    autocomplete = sxng_request.preferences.get_value('autocomplete')

    # chrome/chromium only supports HTTP GET....
    if sxng_request.headers.get('User-Agent', '').lower().find('webkit') >= 0:
        method = 'GET'

    if method not in ('POST', 'GET'):
        method = 'POST'

    # Simple OpenSearch XML for API mode
    opensearch_xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>{settings['general']['instance_name']}</ShortName>
  <Description>Search with SearXNG</Description>
  <Url type="text/html" method="{method}" template="{settings['server'].get('base_url', '')}/search?q={{searchTerms}}"/>
</OpenSearchDescription>'''

    resp = Response(response=opensearch_xml, status=200, mimetype="application/opensearchdescription+xml")
    return resp


@app.route('/config')
def config():
    """Return configuration in JSON format."""
    _engines = []
    for name, engine in engines.items():
        if not sxng_request.preferences.validate_token(engine):
            continue

        _languages = engine.traits.languages.keys()
        _engines.append(
            {
                'name': name,
                'categories': engine.categories,
                'shortcut': engine.shortcut,
                'enabled': not engine.disabled,
                'paging': engine.paging,
                'language_support': engine.language_support,
                'languages': list(_languages),
                'regions': list(engine.traits.regions.keys()),
                'safesearch': engine.safesearch,
                'time_range_support': engine.time_range_support,
                'timeout': engine.timeout,
            }
        )

    _plugins = []
    for _ in searx.plugins.STORAGE:
        _plugins.append({'name': _.id, 'enabled': _.active})

    _limiter_cfg = limiter.get_cfg()

    from searx.locales import LOCALE_NAMES

    return jsonify(
        {
            'categories': list(categories.keys()),
            'engines': _engines,
            'plugins': _plugins,
            'instance_name': settings['general']['instance_name'],
            'locales': LOCALE_NAMES,
            'default_locale': 'en',  # Default for API mode
            'autocomplete': settings['search']['autocomplete'],
            'safe_search': settings['search']['safe_search'],
            'default_theme': 'simple',
            'version': VERSION_STRING,
            'brand': {
                'PRIVACYPOLICY_URL': get_setting('general.privacypolicy_url'),
                'CONTACT_URL': get_setting('general.contact_url'),
                'GIT_URL': GIT_URL,
                'GIT_BRANCH': GIT_BRANCH,
                'DOCS_URL': get_setting('brand.docs_url'),
            },
            'limiter': {
                'enabled': limiter.is_installed(),
                'botdetection.ip_limit.link_token': _limiter_cfg.get('botdetection.ip_limit.link_token'),
                'botdetection.ip_lists.pass_searxng_org': _limiter_cfg.get('botdetection.ip_lists.pass_searxng_org'),
            },
            'doi_resolvers': list(settings.get('doi_resolvers', {}).keys()),
            'default_doi_resolver': settings.get('default_doi_resolver', 'doi.org'),
            'public_instance': settings['server']['public_instance'],
        }
    )


@app.errorhandler(404)
def page_not_found(_e):
    return jsonify({'error': 'Not Found'}), 404


def run():
    """Runs the application on a local development server.

    This run method is only called when SearXNG is started via ``__main__``::

        python -m searx.webapp_api

    Do not use :ref:`run() <flask.Flask.run>` in a production setting.  It is
    not intended to meet security and performance requirements for a production
    server.

    It is not recommended to use this function for development with automatic
    reloading as this is badly supported.  Instead you should be using the flask
    command line script's run support::

        flask --app searx.webapp_api run --debug --reload --host 127.0.0.1 --port 8888

    .. _Flask.run: https://flask.palletsprojects.com/en/stable/api/#flask.Flask.run
    """

    host: str = get_setting("server.bind_address")  # type: ignore
    port: int = get_setting("server.port")  # type: ignore

    if searx.sxng_debug:
        logger.debug("run API-only server (DEBUG) on %s:%s", host, port)
        app.run(
            debug=True,
            port=port,
            host=host,
            threaded=True,
            extra_files=[DEFAULT_SETTINGS_FILE],
        )
    else:
        logger.debug("run API-only server on %s:%s", host, port)
        app.run(port=port, host=host, threaded=True)


def init():

    if searx.sxng_debug or app.debug:
        app.debug = True
        searx.sxng_debug = True

    # check secret_key in production
    if not app.debug and get_setting("server.secret_key") == 'ultrasecretkey':
        logger.error("server.secret_key is not changed. Please use something else instead of ultrasecretkey.")
        sys.exit(1)

    searx.plugins.initialize(app)

    metrics: bool = get_setting("general.enable_metrics")  # type: ignore
    searx.search.initialize(enable_checker=True, check_network=True, enable_metrics=metrics)

    limiter.initialize(app, settings)


app.wsgi_app = ProxyFix(app.wsgi_app)
patch_application(app)

# remove when we drop support for uwsgi
application = app

init()

if __name__ == "__main__":
    run()