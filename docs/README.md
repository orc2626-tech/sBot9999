# Aurora — Docs

- **AURORA_AI_DIRECTIVE_V3.md** — التوجيه الهندسي للـ AI (نسخة 3). ضع نسخة من الملف الذي رفعته هنا باسم `AURORA_AI_DIRECTIVE_V3.md` ليكون مرجعًا لأي مطوّر أو ذكاء اصطناعي يعمل على المشروع.
- أي AI يعمل على هذا الريبو يجب أن يقرأ التوجيه (إن وُجد) قبل أي تغيير في الكود.

تم تنفيذ أجزاء من التوجيه في المشروع الحالي:
- HMAC-SHA256 للتوقيع، get_account، place_order، cancel_order، get_open_orders
- تحليل أحداث User Stream (executionReport، outboundAccountPosition)
- Rate limit tracker، توسيع StateSnapshot (balances، open_orders، rate_limits، recent_errors، last_heartbeat_age_s)
- Reconcile فعلي (get_account + تحديث balances و rate_limit)
- Dead Man's Switch (heartbeat endpoint + فحص كل 60 ثانية)
- Graceful shutdown (Ctrl+C → Pause + حفظ config)
- ربط المنصة: SystemHealthCard، RecentErrorsPanel، PositionManagementGrid من الـ API + إرسال heartbeat كل 5 دقائق
