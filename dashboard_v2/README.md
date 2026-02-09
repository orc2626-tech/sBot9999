# Aurora Pro — Dashboard V2

لوحة تحكم احترافية لبوت التداول Spot على Binance، مطابقة لمرجع الواجهة مع نفس التخطيط والألوان والمكونات.

## التشغيل

```bash
npm install
npm run dev
```

يفتح التطبيق على: **http://localhost:5173**

### ربط المنصة بالبوت (Backend)

لعرض الحقيقة من البوت (UI Truth) وأزرار Pause/Resume/Kill:

1. أنشئ ملف `.env` في مجلد `dashboard_v2/` (انظر `.env.example`).
2. ضع `VITE_API_URL=http://127.0.0.1:3000` (أو عنوان البوت).
3. ضع `VITE_ADMIN_TOKEN=نفس قيمة AURORA_ADMIN_TOKEN` في البوت.

بدون هذه المتغيرات: الهيدر يعرض قيم افتراضية؛ عند فشل الاتصال يظهر بانر خطأ (لا صمت).

## الصفحات (10 صفحات منسّقة بنفس النظام)

| المسار | الوصف |
|--------|--------|
| `/` (Overview) | الشاشة الرئيسية: رموز، إشارات، تأهيل، مراكز، صحة، AI، توأم، أخطاء، سجل |
| `/live` | التداول المباشر + قرارات الدخول |
| `/strategy` | الإشارات، الـ Ensemble، أسباب الرفض |
| `/insurance` | التأمين، نافذة التأكيد، بوابة الحظر |
| `/risk` | الحدود، دوائر القطع، التعرض |
| `/positions` | المراكز، FSM، التوأم، الجداول الزمنية |
| `/execution` | TCA، انزلاق، إعادة المحاولة |
| `/ai` | مراقب AI، الشذوذات، تشغيل تدقيق عميق |
| `/diagnostics` | أخطاء، أعمار WS، سجل المطابقة |
| `/control` | إعدادات وقت التشغيل، أزرار Pause/Resume/Kill، Feature Flags |

## المكونات (مطابقة للصورة)

- **Global Truth Header**: System GREEN, Mode LIVE, WS SYNCED, Recon IN SYNC, Risk NORMAL, Strik VR.2.9, State, Seq
- **Symbols Ticker Row**: BTC/ETH/SOL/BNB/XRP مع أسعار وتغيير % ورسوم مصغرة
- **Signal Ensemble**: Momentum Up, Breakout, Trend Align, RSI Surge + Ensemble Decision BUY
- **Trade Eligibility**: Probability, Risk/Reward, Liquidity, Final Decision ALLOWED
- **Aggregate Positions**: إجمالي مراكز + رسم بياني
- **Position Management Grid**: 5 رموز مع أداء % ورسوم مصغرة
- **System Health**: Uptime, WS Latency, Last Reconcile, Last Error
- **AI Observer**: Anomalies, Last Audit, Recommendation
- **Binance Twin State**: Exchange vs Local, Drift Status
- **Position Timelines**: قائمة بتغييرات المراكز
- **Recent Errors**: آخر الأخطاء مع وقت وكود
- **Action Log**: جدول الإجراءات (Entry, Pause, Resume, Close, P/L)

## إضافة احترافية: Live Decision Pipeline

في الصفحة الرئيسية و Live و Strategy تظهر لوحة **Live Decision Pipeline** تعرض مسار القرار خطوة بخطوة:

1. Data Quality  
2. Regime & Session  
3. Strategy Proposals  
4. Evidence Quorum  
5. Trade Insurance  
6. Risk Pre-Check  
7. Execution Quality (TCA)  
8. Final (ALLOW/REJECT)

كل خطوة تظهر حالة (Pass/Allow/Block) وشرح مختصر، بما يتماشى مع مواصفات AURORA vNext.

## البناء

```bash
npm run build
npm run preview   # معاينة نسخة الإنتاج
```

## التقنيات

- React 18 + TypeScript
- Vite 5
- React Router 6
- Tailwind CSS
- Recharts (رسوم بيانية)
- Lucide React (أيقونات)
