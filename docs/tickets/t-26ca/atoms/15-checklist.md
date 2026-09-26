# Checklist

- [ ] Reproduce the 000 under load (e.g. parallel requests or stress) (i-5cb3)
- [ ] Always write the 431 before closing (drain or shutdown write side first) (i-b2bd)
- [ ] Plant: the oversized request gets 431 N times in a row under load (i-4996)
