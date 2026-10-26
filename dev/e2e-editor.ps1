# Drives the block editor of the dev server (or the reference with -Ref): new post, title, a paragraph and a heading,
# save the draft, publish, update, and prints the REST requests and the problems of the page. The post stays in the
# database: delete it afterwards (the id is printed).
#   dev/e2e-editor.ps1 [-Ref]
param([switch]$Ref)
$js = @'
(async () => {
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  for (let i = 0; i < 40 && !(window.wp && wp.data && wp.data.select('core/editor') && wp.data.select('core/editor').getCurrentPostId()); i++) await wait(500);
  const { dispatch, select } = wp.data;
  const out = {};
  dispatch('core/editor').editPost({ title: 'E2E editor test' });
  dispatch('core/block-editor').resetBlocks([wp.blocks.createBlock('core/paragraph', { content: 'Hello from the e2e test' }), wp.blocks.createBlock('core/heading', { content: 'A heading', level: 2 })]);
  await dispatch('core/editor').savePost();
  await wait(1500);
  const id = select('core/editor').getCurrentPostId();
  out.draft = select('core/editor').getCurrentPost().status;
  dispatch('core/editor').editPost({ status: 'publish' });
  await dispatch('core/editor').savePost();
  await wait(1500);
  out.published = select('core/editor').getCurrentPost().status;
  out.link = select('core/editor').getCurrentPost().link;
  dispatch('core/editor').editPost({ title: 'E2E editor test 2', excerpt: 'An excerpt' });
  await dispatch('core/editor').savePost();
  await wait(1500);
  out.title = select('core/editor').getCurrentPost().title;
  out.error = select('core').getLastEntitySaveError('postType', 'post', id);
  out.id = id;
  return JSON.stringify(out);
})()
'@
$shot = @{ Page = 'post-new.php'; Out = "$env:TEMP\e2e_editor.png"; Wait = 8; Log = $true; Eval = $js }
if ($Ref) { $shot.Ref = $true }
& (Join-Path $PSScriptRoot 'shot.ps1') @shot
