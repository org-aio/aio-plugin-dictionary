"""在独立本机 PostgreSQL 和真实 Docker Sidecar 中验收字典整包生命周期。"""
import hashlib
import json
import os
import socket
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

plugin = Path(__file__).resolve().parents[2]
binary = Path(os.environ['AIO_DICTIONARY_TEST_HOST']).resolve(strict=True)
database = os.environ['AIO_DICTIONARY_TEST_DATABASE_URL']
parsed = urllib.parse.urlparse(database)
assert parsed.hostname == '127.0.0.1' and parsed.path == '/dictionary_delivery_test'
postgres = os.environ['AIO_DICTIONARY_TEST_POSTGRES_CONTAINER']
web = Path(os.environ['AIO_DICTIONARY_TEST_WEB']).resolve(strict=True)
packager = Path(os.environ['AIO_DICTIONARY_TEST_PACKAGER']).resolve(strict=True)
root = Path(tempfile.mkdtemp(prefix='aio-dictionary-delivery-'))
(root / 'processes').mkdir()
(root / 'config.toml').write_text('plugins = []\n', encoding='utf-8')
with socket.socket() as reservation:
    reservation.bind(('127.0.0.1', 0))
    port = reservation.getsockname()[1]
base = 'http://127.0.0.1:%d' % port
image = 'sha256:48a661b5e5f4560287dfc254e948825538d0f7a1379616495a424ed375afd31f'
env = {k: v for k, v in os.environ.items() if not k.startswith('AIO_')}
env.update({
    'AIO_DATABASE_URL': database, 'AIO_COMPONENT_DATABASE_URL': database,
    'AIO_COMPONENT_HOME': str(root / 'components'), 'AIO_PLUGIN_CACHE': str(root / 'cache'),
    'AIO_CONFIG': str(root / 'config.toml'), 'AIO_FILE_STORAGE_DIR': str(root / 'files'),
    'AIO_PROCESS_ROOT': str(root / 'processes'), 'AIO_PROCESS_IMAGES': image,
    'AIO_PLUGIN_SUPERVISOR_SOCKET': str(root / 'supervisor.sock'),
    'AIO_PUBLIC_ORIGIN': base, 'AIO_WEB_HOST': '127.0.0.1', 'AIO_WEB_PORT': str(port),
    'AIO_WEB_DIST': str(web), 'AIO_BOOTSTRAP_ACCOUNT': 'dictionary-test',
    'AIO_BOOTSTRAP_PASSWORD': uuid.uuid4().hex, 'AIO_PLUGIN_PUBLISH_ACCOUNTS': 'dictionary-test',
})
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
cookie = None
host = None
supervisor = None
host_log = open(str(root / 'host.log'), 'w', encoding='utf-8')
supervisor_log = open(str(root / 'supervisor.log'), 'w', encoding='utf-8')
host_hash = hashlib.sha256(binary.read_bytes()).hexdigest()


def sql(statement):
    return subprocess.check_output(['docker', 'exec', '-i', postgres, 'psql', '-U', parsed.username, '-d', 'dictionary_delivery_test', '-X', '-v', 'ON_ERROR_STOP=1', '-Atc', statement], universal_newlines=True).strip()


def request(path, value=None, raw=None):
    headers = {'Cookie': cookie} if cookie else {}
    body = raw
    if raw is not None:
        headers['Content-Type'] = 'application/vnd.aio.component+gzip'
    if value is not None:
        body = json.dumps(value).encode('utf-8')
        headers['Content-Type'] = 'application/json'
    try:
        response = http.open(urllib.request.Request(base + path, data=body, headers=headers), timeout=180)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def success(path, value=None, raw=None):
    status, _, body = request(path, value, raw)
    assert 200 <= status < 300, '%s: HTTP %s %s' % (path.split('/')[3:5], status, body[:250])
    if not body:
        return None
    return json.loads(body).get('data')


def until(predicate, timeout=60):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise AssertionError('等待超时；检查日志目录 ' + str(root))


def ready():
    assert host.poll() is None, '宿主提前退出；检查 ' + str(root)
    try:
        return request('/health')[0] == 200
    except OSError:
        return False


def launch():
    global host
    host = subprocess.Popen([str(binary)], cwd=str(root), env=env, stdout=host_log, stderr=subprocess.STDOUT)
    until(ready)


def stop():
    global host
    if host is not None and host.poll() is None:
        host.terminate()
        assert host.wait(timeout=40) == 0
    host = None


def pages():
    return success('/api/runtime/catalog')['pages']


def mount(source, version=None):
    value = success('/api/runtime/frontend/mount', {'page_id': 'component:%s:dictionary-management' % source})
    if version is not None:
        path = urllib.parse.urlsplit(value['src'])
        status, _, body = request(path.path + ('?' + path.query if path.query else ''))
        assert status == 200 and ('data-delivery-version="%s"' % version).encode() in body
    return value['token']


def invoke(token, method, path, value=None):
    payload = {'method': method, 'path': path, 'headers': [], 'body': []}
    if value is not None:
        payload['headers'] = [{'name': 'content-type', 'value': 'application/json'}]
        payload['body'] = list(json.dumps(value).encode('utf-8'))
    response = success('/api/runtime/components/%s/request' % token, payload)
    assert 200 <= response['status'] < 300, response
    return json.loads(bytes(response['body']).decode('utf-8'))['data'] if response['body'] else None


def publish(version):
    bundle = root / ('dictionary-%s.aio-plugin' % version)
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=str(plugin), universal_newlines=True).strip()
    candidate = root / ('candidate-' + version)
    candidate.mkdir()
    shutil.copy2(str(plugin / 'aio-plugin.toml'), str(candidate / 'aio-plugin.toml'))
    shutil.copytree(str(plugin / 'dist'), str(candidate / 'dist'))
    shutil.copytree(str(plugin / 'server/migrations'), str(candidate / 'server/migrations'))
    index = candidate / 'dist/frontend/index.html'
    index.write_text(index.read_text(encoding='utf-8').replace('<body>', '<body data-delivery-version="%s">' % version), encoding='utf-8')
    subprocess.check_call([str(packager), str(candidate), 'aio-plugin.toml', 'https://github.com/org-aio/aio-plugin-dictionary.git', commit, version, str(bundle)])
    result = success('/api/runtime/components/publish', raw=bundle.read_bytes())
    print('Published dictionary ' + version, flush=True)
    return result


try:
    assert sql('SELECT count(*) FROM pg_tables WHERE schemaname=\'public\'') == '0', '请使用新的专用测试库'
    supervisor = subprocess.Popen([str(binary), 'supervisor'], cwd=str(root), env=env, stdout=supervisor_log, stderr=subprocess.STDOUT)
    until(lambda: (root / 'supervisor.sock').exists())
    launch()
    status, headers, _ = request('/api/auth/login', {'account': env['AIO_BOOTSTRAP_ACCOUNT'], 'password': env['AIO_BOOTSTRAP_PASSWORD']})
    assert status == 200
    cookie = headers['Set-Cookie'].split(';')[0]
    assert request('/api/dictionaries')[0] == 404
    first = publish('0.1.0')
    entries = success('/api/runtime/marketplace')
    entry = next(x for x in entries if x['git'] == 'https://github.com/org-aio/aio-plugin-dictionary.git')
    source = entry['source_id']
    assert not entry['installed']
    success('/api/runtime/plugins/install', {'git': entry['git']})
    assert any(p['id'] == 'component:%s:dictionary-management' % source for p in pages())
    token = mount(source, '0.1.0')
    sql((plugin / 'server/migrations/001_dictionary.sql').read_text(encoding='utf-8'))
    sql("INSERT INTO public.dictionary_types(tenant_id,id,code,name) VALUES('default','legacy-dictionary','legacy','Legacy')")
    migration_env = dict(env, AIO_LEGACY_DATABASE_URL=database)
    subprocess.check_call([os.environ['AIO_DICTIONARY_TEST_MIGRATOR'], 'default'], env=migration_env)
    assert any(t['id'] == 'legacy-dictionary' for t in invoke(token, 'GET', '/api/dictionaries')['types'])
    assert sql("SELECT count(*) FROM public.dictionary_types WHERE id='legacy-dictionary'") == '1'
    created = invoke(token, 'POST', '/api/dictionaries/types', {'code': 'durable', 'name': '独立整包', 'description': '', 'enabled': True})
    identifier = created['id']
    print('Installed and called actual Sidecar', flush=True)

    browser = os.environ.get('AIO_DICTIONARY_TEST_BROWSER_CONTAINER')
    if browser:
        browser_env = dict(env, AIO_URL=base)
        subprocess.check_call(['docker', 'exec', '-w', '/workspace', '-e', 'AIO_URL', '-e', 'AIO_BOOTSTRAP_ACCOUNT', '-e', 'AIO_BOOTSTRAP_PASSWORD', browser, 'node', 'tests/browser/dictionary.cjs'], env=browser_env)

    second = publish('0.1.1')
    success('/api/runtime/plugins/install', {'git': entry['git']})
    assert first['revision'] != second['revision']
    token = mount(source, '0.1.1')
    assert any(t['id'] == identifier for t in invoke(token, 'GET', '/api/dictionaries')['types'])
    success('/api/runtime/plugins/%s/rollback' % source, {})
    token = mount(source, '0.1.0')
    assert any(t['id'] == identifier for t in invoke(token, 'GET', '/api/dictionaries')['types'])
    current = next(x for x in success('/api/runtime/marketplace') if x['source_id'] == source)
    assert current['active_revision'] == first['revision']
    success('/api/runtime/plugins/%s/disable' % source, {})
    assert not any(p['id'] == 'component:%s:dictionary-management' % source for p in pages())
    assert request('/api/runtime/components/%s/request' % token, {'method': 'GET', 'path': '/api/dictionaries'})[0] >= 400
    stop()
    launch()
    assert not any(p['id'] == 'component:%s:dictionary-management' % source for p in pages())
    success('/api/runtime/plugins/%s/enable' % source, {})
    token = mount(source, '0.1.0')
    current = next(x for x in success('/api/runtime/marketplace') if x['source_id'] == source)
    assert current['active_revision'] == first['revision']
    assert sql("SELECT excluded_digest FROM component_installations WHERE source_id='%s' AND tenant_id='default'" % source) == second['revision']
    assert any(t['id'] == identifier for t in invoke(token, 'GET', '/api/dictionaries')['types'])
    grants = sql("WITH removed AS (DELETE FROM role_permissions WHERE tenant_id='default' AND permission='dictionary:manage' RETURNING *) SELECT json_agg(removed) FROM removed")
    assert json.loads(grants), '需要保存原角色授权，以便撤权验收后原样恢复'
    assert not any(p['id'] == 'component:%s:dictionary-management' % source for p in pages())
    assert request('/api/runtime/components/%s/request' % token, {'method': 'GET', 'path': '/api/dictionaries'})[0] >= 400
    sql("INSERT INTO role_permissions SELECT * FROM json_populate_recordset(NULL::role_permissions, '%s'::json) ON CONFLICT DO NOTHING" % grants.replace("'", "''"))
    token = mount(source)
    assert any(t['id'] == identifier for t in invoke(token, 'GET', '/api/dictionaries')['types'])
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == host_hash, '测试期间不能重编宿主'
    success('/api/runtime/plugins/%s/uninstall' % source, {})
    assert not any(p['id'] == 'component:%s:dictionary-management' % source for p in pages())
    report = {'installed': True, 'upgraded': True, 'rolledBack': True, 'disabledAcrossRestart': True, 'rollbackPreservedOnEnable': True, 'dataRetained': True, 'legacyDataCopied': True, 'frontendRevisionVerified': True, 'liveRevocation': True, 'hostUnchanged': True, 'logs': str(root)}
finally:
    try:
        stop()
    finally:
        if supervisor is not None and supervisor.poll() is None:
            supervisor.terminate()
            supervisor.wait(timeout=20)
        host_log.close()
        supervisor_log.close()

report['gracefulShutdown'] = True
print(json.dumps(report), flush=True)
