const assert = require('node:assert/strict');
const { mkdir, writeFile } = require('node:fs/promises');
const { resolve } = require('node:path');
const { randomUUID } = require('node:crypto');
const { chromium } = require('playwright');

const base = process.env.AIO_URL;
assert(new URL(base).hostname === '127.0.0.1', 'Only the isolated local test host is allowed');
const output = resolve('test-results');

async function scenario(browser, mobile) {
  const context = await browser.newContext({ viewport: mobile ? { width: 390, height: 844 } : { width: 1440, height: 1000 }, isMobile: mobile, hasTouch: mobile });
  const page = await context.newPage();
  const errors = [];
  let frame;
  async function rpc(method, path, value) {
    return frame.locator('body').evaluate((_, {method,path,value}) => window.aioPlugin.json(method,path,value), {method,path,value});
  }
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  const request = context.request;
  const name = `验收字典-${randomUUID().slice(0, 8)}`;
  const code = `test-${randomUUID()}`;
  let typeId;
  async function view() {
    return (await rpc('GET', '/api/dictionaries')).data.types.find(type => type.id === typeId);
  }
  async function save(dialog, method, path) {
    const result = page.waitForResponse(response => response.url().endsWith('/request') && response.request().postDataJSON()?.path === path && response.request().postDataJSON()?.method === method);
    await dialog.getByRole('button', { name: '保存', exact: true }).click();
    const response = await result;
    assert(response.ok(), await response.text());
    await dialog.waitFor({ state: 'detached' });
    const transport = (await response.json()).data;
    assert(transport.status < 400, JSON.stringify(transport));
    return JSON.parse(Buffer.from(transport.body).toString()).data;
  }
  try {
    assert((await request.post(`${base}/api/auth/login`, { data: { account: process.env.AIO_BOOTSTRAP_ACCOUNT, password: process.env.AIO_BOOTSTRAP_PASSWORD } })).ok());
    assert.equal((await request.get(`${base}/api/dictionaries`)).status(), 404, '宿主不应再注册字典业务路由');
    await page.goto(base);
    await page.locator('.application-shell:visible').waitFor();
    await page.getByRole('navigation', {name:'场景'}).getByRole('button', {name:'系统',exact:true}).click();
    if (mobile) await page.getByRole('button', { name: '打开菜单', exact: true }).click();
    const nav = mobile ? page.getByRole('dialog') : page.locator('.application-shell__sidebar');
    await nav.getByRole('button', { name: '字典管理', exact: true }).click();
    frame = page.frameLocator('iframe[title="字典管理"]');
    await frame.getByRole('heading', { name: '字典管理', exact: true }).waitFor();
    await frame.locator('.admin-page').evaluate(async element => {
      await document.fonts.ready;
      if (getComputedStyle(element).display !== 'flex') throw new Error('共享页面样式未加载');
    });
    await frame.getByRole('button', { name: '新建类型', exact: true }).click();
    const typeDialog = frame.getByRole('dialog', { name: '新建字典类型', exact: true });
    await typeDialog.getByLabel('字典类型编码', { exact: true }).fill(code);
    await typeDialog.getByLabel('字典类型名称', { exact: true }).fill(name);
    await typeDialog.getByLabel('字典类型说明', { exact: true }).fill('实际接口增删改查验证');
    typeId = (await save(typeDialog, 'POST', '/api/dictionaries/types')).id;
    await frame.getByRole('searchbox', { name: '搜索字典类型', exact: true }).fill(name);
    await frame.getByRole('listbox', { name: '字典类型', exact: true }).getByRole('option').filter({ hasText: name }).getByRole('button').click();
    await frame.getByRole('heading', { name, exact: true }).waitFor();

    await frame.getByRole('button', { name: '编辑类型', exact: true }).click();
    const editType = frame.getByRole('dialog', { name: '编辑字典类型', exact: true });
    await editType.getByLabel('字典类型说明', { exact: true }).fill('已修改类型说明');
    await save(editType, 'PUT', `/api/dictionaries/types/${typeId}`);
    assert.equal((await view()).description, '已修改类型说明');

    async function add(value, label, isDefault) {
      await frame.getByRole('button', { name: '新建字典项', exact: true }).click();
      const dialog = frame.getByRole('dialog', { name: '新建字典项', exact: true });
      await dialog.getByLabel('字典值', { exact: true }).fill(value);
      await dialog.getByLabel('字典项标签', { exact: true }).fill(label);
      if (isDefault) await dialog.getByRole('checkbox', { name: '默认项', exact: true }).click();
      return save(dialog, 'POST', '/api/dictionaries/items');
    }
    const first = await add('first', '第一项', true);
    const second = await add('second', '第二项', true);
    let data = await view();
    assert.equal(data.items.find(item => item.id === first.id).is_default, false);
    assert.equal(data.items.find(item => item.id === second.id).is_default, true);
    const table = frame.getByRole('table', { name: '字典项', exact: true });
    await table.getByRole('button', { name: '编辑 第一项', exact: true }).click();
    const edit = frame.getByRole('dialog', { name: '编辑字典项', exact: true });
    await edit.getByLabel('字典项标签', { exact: true }).fill('第一项已编辑');
    await edit.getByLabel('字典项排序', { exact: true }).fill('7');
    await edit.getByRole('checkbox', { name: '启用字典项', exact: true }).click();
    await save(edit, 'PUT', `/api/dictionaries/items/${first.id}`);
    data = await view();
    assert.equal(data.items.find(item => item.id === first.id).enabled, false);
    assert.equal(data.items.find(item => item.id === first.id).sort_order, 7);

    await frame.getByRole('button', { name: '字典项状态', exact: true }).click();
    await frame.getByRole('option', { name: '停用', exact: true }).click();
    await table.getByText('第一项已编辑', { exact: true }).waitFor();
    assert.equal(await table.getByText('第二项', { exact: true }).count(), 0);
    await frame.getByRole('button', { name: '字典项状态', exact: true }).click();
    await frame.getByRole('option', { name: '全部状态', exact: true }).click();
    await frame.getByRole('searchbox', { name: '搜索字典项', exact: true }).fill('second');
    await table.getByText('第二项', { exact: true }).waitFor();
    assert.equal(await table.getByText('第一项已编辑', { exact: true }).count(), 0);
    await frame.getByRole('searchbox', { name: '搜索字典项', exact: true }).fill('');
    await frame.getByRole('button', { name: '排序：排序', exact: true }).click();
    assert.equal(await table.locator('th[aria-sort="ascending"]').count(), 1);

    // 批量删除中的失败项保留在对话框里；重试不能重复删除已经成功的项。
    const deletionCalls = [];
    let rejectFirst = true;
    await page.route('**/api/runtime/components/*/request', route => {
      const input = route.request().postDataJSON();
      if (input?.method !== 'DELETE' || !input.path.startsWith('/api/dictionaries/items/')) return route.continue();
      const id = input.path.split('/').pop();
      deletionCalls.push(id);
      if (rejectFirst && id === first.id) return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({data:{status:503,headers:[],body:Array.from(Buffer.from(JSON.stringify({error:'部分删除失败，请重试'})))}}) });
      return route.continue();
    });
    await table.getByRole('checkbox', { name: '选择当前页', exact: true }).click();
    await frame.getByRole('button', { name: '删除选中 (2)', exact: true }).click();
    const deletion = frame.getByRole('dialog', { name: '删除字典项', exact: true });
    await deletion.getByRole('button', { name: '确认删除', exact: true }).click();
    await deletion.getByRole('alert').filter({ hasText: '部分删除失败' }).waitFor();
    assert.deepEqual((await view()).items.map(item => item.id), [first.id]);
    rejectFirst = false;
    await deletion.getByRole('button', { name: '确认删除', exact: true }).click();
    await deletion.waitFor({ state: 'detached' });
    assert.equal(deletionCalls.filter(id => id === first.id).length, 2);
    assert.equal(deletionCalls.filter(id => id === second.id).length, 1);
    assert.equal((await view()).items.length, 0);
    await page.unroute('**/api/runtime/components/*/request');
    await add('cascade', '随类型一起删除', false);
    const cascade = table.getByText('随类型一起删除', { exact: true });
    await cascade.waitFor();
    await cascade.scrollIntoViewIfNeeded();
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Dictionary host overflow');
    assert(await frame.locator('body').evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Dictionary iframe overflow');
    await page.screenshot({ path: resolve(output, `${mobile ? 'mobile' : 'desktop'}-dictionary.png`) });
    await frame.getByRole('button', { name: '删除类型', exact: true }).click();
    const deleteType = frame.getByRole('dialog', { name: '删除字典类型', exact: true });
    await deleteType.getByText('同时删除该类型下的 1 个字典项，无法撤销。', { exact: true }).waitFor();
    await deleteType.getByRole('button', { name: '取消', exact: true }).click();
    assert(await view());
    await frame.getByRole('button', { name: '删除类型', exact: true }).click();
    await deleteType.getByRole('button', { name: '确认删除', exact: true }).click();
    await deleteType.waitFor({ state: 'detached' });
    assert.equal(await view(), undefined);
    assert.deepEqual(errors, []);
    return { viewport: mobile ? 'mobile' : 'desktop', typeCrud: true, itemCrud: true, defaultExclusive: true, filter: true, search: true, sort: true, partialFailureRetry: true, cascadeConfirmation: true };
  } catch (error) {
    await page.screenshot({ path: resolve(output, 'dictionary-failure.png') });
    if (frame) await writeFile(resolve(output, 'dictionary-layout.json'), JSON.stringify(await frame.locator('body').evaluate(() => ({
      width: innerWidth, scrollWidth: document.documentElement.scrollWidth,
      links: [...document.querySelectorAll('link')].map(link => ({ href: link.href, loaded: !!link.sheet })),
      regions: [...document.querySelectorAll('.admin-page,.admin-split,.admin-section,.admin-collection,.admin-table,.data-table-workspace')].map(element => ({
        class: element.className, bounds: element.getBoundingClientRect().toJSON(), display: getComputedStyle(element).display,
      })),
    })), null, 2));
    console.error(errors);
    throw error;
  } finally {
    if (typeId) await rpc('DELETE', `/api/dictionaries/types/${typeId}`).catch(() => {});
    await context.close();
  }
}

(async () => {
  await mkdir(output, { recursive: true });
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
  try {
    const report = [await scenario(browser, false), await scenario(browser, true)];
    await writeFile(resolve(output, 'dictionary-report.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, null, 2));
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
