# Queue browser observation

Observed at source 5456602224ec69811a6a1dbbcf0145c7ce2631f7 using the in-app
browser against a disposable local fixture. No model, provider or paid calls.

1. Unlock and select saved Simpler signup revision 1.
2. Queue saved revision; UI shows queued, then waiting for agent.
3. Cancel queued work; UI shows cancelled and zero attempts.
4. Queue same source again. Enable the synthetic availability marker.
5. UI automatically shows dispatched. View attempt displays one completed, stopped
   attempt with required checks and unverified labeling.
6. Lock session; queue and application controls disappear.

Warrant UUID: de2fff6c-faaa-44dc-a952-e139af4026c6.
Completed attempt: 32db4b34-3932-4091-a6e7-2841905363ad.
Retained queue and execution JSON records are in browser-fixture/. Temporary paths
identify the fixture that ran; they are not live deployment or lineage endpoints.
The fixture was stopped and its browser tab closed. User tabs/services untouched.

Polling replaced unchanged button nodes. A live locator completed cancellation;
subsequent b908185 caches unchanged queue snapshots to preserve focus. That patch
has syntax evidence; follow-up direct focus observation remains pending. Optional
project board, hotline and verifier were absent in this tiny repository fixture.
This observation does not qualify those services or establish external isolation.

Follow-up at b908185 (with later evidence-only commit present): fresh fixture
50f9aed3-7e4d-49fe-a581-dd19a778444a queued to waiting-for-agent. Keyboard Tab moved
focus to Cancel queued work. Later read-only DOM inspection after the polling
interval still reported that button as document.activeElement, with waiting state
unchanged. Enter cancelled the request. Fixture tab closed. This settles the
previously pending direct focus observation for unchanged queue data.
