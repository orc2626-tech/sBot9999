# Aurora Bot — RUNBOOK

## تشغيل البوت + المنصة

> **دليل مفصل بالعربية:** انظر ملف **كيف_تشغّل_البوت_والمنصة.md**

### 0) تشغيل بنقرة واحدة (Windows)

من جذر المشروع:

```powershell
.\start.ps1
```

يشغّل البوت في نافذة منفصلة، ينتظر جاهزية الـ API، ثم يشغّل المنصة. يُنشئ `.env` من `.env.example` إن لم يكن موجوداً.

### 1) البوت (Backend)

```bash
cd backend
copy .env.example .env
# عدّل .env: AURORA_ADMIN_TOKEN؛ (اختياري) AURORA_API_LISTEN=127.0.0.1:3001 إذا 3000 مشغول
cargo run
```

- الـ API يعمل على: `http://127.0.0.1:3000` (أو المنفذ في AURORA_API_LISTEN)
- بدون `AURORA_ADMIN_TOKEN`: endpoints المحمية ترجع 403 (مقصود).
- بدون Binance keys: User Stream معطّل؛ البوت يعمل بدون تداول فعلي.

### 2) المنصة (Dashboard)

```bash
cd dashboard_v2
npm install
npm run dev
```

- المنصة تعمل على: `http://localhost:5173`
- لربطها بالبوت: أنشئ `.env` في `dashboard_v2/`:
  - `VITE_API_URL=http://127.0.0.1:3001` (أو نفس منفذ البوت)
  - `VITE_ADMIN_TOKEN=نفس قيمة AURORA_ADMIN_TOKEN`
- بدون هذه المتغيرات: الهيدر يعرض قيم افتراضية ويظهر بانر إذا فشل الاتصال بالـ API.

### 3) التحقق السريع

```bash
# بدون توكن — health يعمل
curl -s http://127.0.0.1:3001/api/v1/health
# /state بدون توكن → 403
curl -s http://127.0.0.1:3001/api/v1/state

# مع توكن
curl -s -H "Authorization: Bearer YOUR_TOKEN" http://127.0.0.1:3001/api/v1/state
curl -s -X POST -H "Authorization: Bearer YOUR_TOKEN" http://127.0.0.1:3001/api/v1/control/pause
```

## هيكل المشروع

- `backend/` — Rust: API، RuntimeConfig، Binance User Stream، Pause/Resume/Kill
- `dashboard_v2/` — React: واجهة Truth، صفحات، تحكم
- `AGENTS.md` — قواعد العمل للـ AI والتطوير

## أمان

- لا ترفع ملف `.env` أو توكنات.
- في بيئة عامة: استخدم HTTPS أو اربط البوت على localhost فقط.

## Dead Man's Switch + وضع Live

- **Dead Man's Switch:** إذا لم يصل heartbeat من المنصة لمدة 30 دقيقة، البوت يضع `no_go_reason: DEAD_MAN_SWITCH` ويحوّل `trading_mode` تلقائياً إلى **Paused** ويحفظ الـ config. لا أوامر جديدة حتى يعيد المستخدم التشغيل أو يرسل heartbeat.
- **التبديل إلى الحساب الحقيقي (Live):** من صفحة Control في المنصة، اختيار "Live" يتطلب **تأكيد صريح** (confirm_live). لا يتم التبديل إلى تداول حقيقي بدون هذا التأكيد.
