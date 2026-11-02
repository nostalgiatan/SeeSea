# Offline tuning corpus: per query one answer page (grade 2), one partial page (grade 1), a bait page whose
# title echoes the query over spam, a keyword-stuffed page, and other topics' answers as distractors.
import json, sys
PORT = 18765
SPAM = ["Cheap watches and handbags at discount prices with free shipping on every order placed today while stocks last at our warehouse outlet.",
        "Sign up to our newsletter to receive exclusive deals, coupons and promotions every week straight to your inbox with no obligation to buy.",
        "Customers who viewed this item also bought sunglasses, phone cases and leather wallets. Limited time offer, buy two and get one free."]
T = [
 dict(q="how does the tokio scheduler work", lang="en",
  A=("Inside the Tokio scheduler", ["The Tokio multi-threaded scheduler runs async tasks on a pool of worker threads. Each worker owns a local run queue, and idle workers steal half of the tasks from a busy worker's queue, which is why it is called a work-stealing scheduler.",
     "When a task is spawned from inside a worker it goes into that worker's local queue, with the most recently woken task kept in a LIFO slot to improve cache locality. A global injection queue holds tasks spawned from outside the runtime, and workers check it periodically so those tasks are not starved.",
     "A task yields back to the scheduler at every await point that returns pending. Tokio also gives each task a cooperative budget so one busy task cannot monopolise a worker thread."]),
  B=("Async programming in Rust", ["Asynchronous Rust lets a program handle many operations at once without a thread per operation. Futures describe work that may not be ready yet, and an executor polls them until they complete.",
     "Libraries such as Tokio and async-std provide executors, timers and network types. Choosing a runtime depends on the ecosystem you build on and how much control you need over threading."])),
 dict(q="why does python have a global interpreter lock", lang="en",
  A=("The reason for the GIL", ["CPython has a global interpreter lock because its memory management uses reference counting, and the counts are not thread safe. Instead of protecting every object with its own lock, the interpreter lets only one thread execute Python bytecode at a time.",
     "The lock made the interpreter simple and fast for single threaded programs and made it easy to write C extensions. The cost is that CPU bound Python threads cannot run in parallel, so people use multiprocessing or native extensions for parallel number crunching.",
     "Recent proposals allow building CPython without the lock, replacing it with finer grained synchronisation and biased reference counting."]),
  B=("Threading in Python", ["The threading module creates operating system threads in Python. Threads are useful for programs that wait on files or sockets because waiting threads release the interpreter.",
     "For background work you can use thread pools from concurrent.futures, which provide a simple interface for submitting tasks and collecting results."])),
 dict(q="postgres vacuum dead tuples bloat", lang="en",
  A=("How VACUUM reclaims space in PostgreSQL", ["PostgreSQL uses multiversion concurrency control, so an UPDATE or DELETE leaves the old row version behind as a dead tuple until no transaction can see it any more. Dead tuples take up space in tables and indexes, which is called bloat.",
     "VACUUM scans the table, marks the space of dead tuples as reusable and updates the visibility map. Plain VACUUM does not shrink the file, but it lets later inserts reuse the space, while VACUUM FULL rewrites the table and takes an exclusive lock.",
     "Autovacuum runs these cleanups in the background; tuning its thresholds and cost limits is the usual way to keep bloat under control on busy tables."]),
  B=("Backing up a PostgreSQL database", ["The pg_dump tool writes a logical backup of one database, while pg_basebackup copies the whole cluster at the file level so it can be restored together with archived WAL.",
     "Test restores regularly. A backup that was never restored is only a hope, and recovery time matters as much as the backup itself."])),
 dict(q="kubernetes pod eviction memory pressure", lang="en",
  A=("Node-pressure eviction in Kubernetes", ["When a node runs low on memory the kubelet starts evicting pods to reclaim it. It compares memory.available against the configured eviction thresholds, and a soft threshold waits for a grace period while a hard threshold evicts immediately.",
     "Pods are ranked for eviction by whether their memory usage exceeds their requests, then by their priority, then by how far above the request they are. BestEffort pods that use memory without any request are therefore evicted first, and Guaranteed pods last.",
     "Setting accurate memory requests and limits, and giving critical workloads a high priority class, keeps important pods from being chosen when the node is under pressure."]),
  B=("Kubernetes resource requests and limits", ["Each container can declare how much CPU and memory it requests and the maximum it may use. The scheduler places pods on nodes using the requests.",
     "Limits are enforced at runtime: a container that goes over its CPU limit is throttled, one that exceeds its memory limit is killed."])),
 dict(q="红烧肉怎么做才软糯", lang="zh",
  A=("软糯红烧肉的做法与火候", ["做红烧肉要选带皮五花肉，切成三厘米见方的块，冷水下锅焯水去血沫。炒糖色时用小火，把冰糖炒到琥珀色再下肉块翻炒上色。",
     "加入热水没过肉块，放葱姜、八角、桂皮和料酒，大火烧开后转小火慢炖一个半小时左右，这是肉变得软糯的关键。最后开大火收汁，让汤汁浓稠挂在肉上。",
     "不要中途加冷水，也不要太早放盐，否则肉容易发柴。"]),
  B=("家常菜入门指南", ["家常菜讲究火候和调味，新手可以从炒青菜、蒸蛋和番茄炒蛋开始练习。",
     "准备好常用调料，如生抽、老抽、料酒和白糖，就能做出大部分家常口味。"])),
 dict(q="光合作用的光反应和暗反应", lang="zh",
  A=("光反应与暗反应的区别", ["光合作用分为光反应和暗反应两个阶段。光反应发生在叶绿体的类囊体薄膜上，叶绿素吸收光能，把水分解为氧气和还原氢，同时生成ATP。",
     "暗反应也叫碳反应，发生在叶绿体基质中，不直接需要光。二氧化碳先被固定成三碳化合物，再利用光反应产生的ATP和还原氢还原为糖类。",
     "光反应为暗反应提供能量和还原力，暗反应又为光反应提供ADP和NADP，两者相互依赖。"]),
  B=("植物是怎样生长的", ["植物通过根吸收水分和无机盐，通过叶片进行光合作用制造有机物，再把养分运输到各个器官。",
     "光照、温度、水分和土壤肥力都会影响植物的生长速度。"])),
 dict(q="how to make braised pork belly", lang="x",
  A=("红烧肉的做法", ["红烧肉用带皮五花肉制作：先切块焯水，再用冰糖炒出琥珀色的糖色，把肉块炒上色后加入热水、酱油、料酒、八角和葱姜。",
     "大火烧开后转小火慢炖约一个半小时，直到肉块软糯入味，最后大火收汁即可。"]),
  B=("Roast pork recipes for the weekend", ["Pork shoulder roasted low and slow becomes tender and easy to shred. Season it with salt and spices and roast it for several hours.",
     "Serve the roast with potatoes and a simple salad, and keep the juices to make gravy."])),
 dict(q="how to fix a car that will not start", lang="en",
  A=("Diagnosing an automobile engine that fails to turn over", ["When an engine cranks slowly or only clicks, the battery is the most likely cause; check the terminals for corrosion and test the voltage, which should be about twelve volts at rest.",
     "If the engine spins but never fires, suspect the fuel pump, a clogged filter or a failed ignition coil. Listen for the pump humming when the key is turned to the on position.",
     "A completely silent starter points to a bad starter motor, a loose ground cable or a dead relay. Jump starting or tapping the starter can confirm which one it is."]),
  B=("Routine car maintenance schedule", ["Change the engine oil and filter at the interval in the owner's manual, rotate the tyres and check the brake pads at every service.",
     "Keeping a log of maintenance makes it easier to spot problems early and helps when selling the vehicle."])),
]
def pages():
    P = {}
    for i, t in enumerate(T):
        a, b = t["A"], t["B"]
        P[f"/a{i}"] = a
        P[f"/b{i}"] = b
        P[f"/bait{i}"] = (t["q"].title() + " - Best Guide 2025!", SPAM)
        words = t["q"]
        P[f"/stuff{i}"] = ("Notes", [(words + ". ") * 12, "More about " + words + " here, " + words + " there, " + words + " everywhere."])
    return P
def results(i, base):
    t = T[i]; n = len(T)
    d1, d2 = (i + 1) % n, (i + 3) % n
    order = [f"/bait{i}", f"/stuff{i}", f"/a{d1}", f"/b{i}", f"/a{d2}", f"/a{i}"]
    titles = pages()
    out = []
    for k, path in enumerate(order):
        out.append({"url": base + path, "title": titles[path][0], "content": titles[path][1][0][:120],
                    "engines": ["bing"] if "bait" in path or "stuff" in path else ["google", "brave"], "positions": [k + 1], "category": "general"})
    return out
if __name__ == "__main__":
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
    from urllib.parse import urlparse, parse_qs
    base = f"http://127.0.0.1:{PORT}"; P = pages()
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a): pass
        def do_GET(self):
            u = urlparse(self.path)
            if u.path == "/search":
                q = parse_qs(u.query).get("q", [""])[0]
                idx = [i for i, t in enumerate(T) if t["q"] == q]
                body = json.dumps({"query": q, "number_of_results": 0, "results": results(idx[0], base) if idx else [], "suggestions": [], "unresponsive_engines": []}).encode()
                self.send_response(200); self.send_header("Content-Type", "application/json")
            elif u.path in P:
                title, paras = P[u.path]
                lang = "zh" if any("一" <= c <= "鿿" for c in title + paras[0]) else "en"
                html = f"<html lang='{lang}'><head><title>{title}</title></head><body><nav><a href='/'>Home</a></nav><article><h1>{title}</h1>" + "".join(f"<p>{p}</p>" for p in paras) + "</article><footer>Copyright</footer></body></html>"
                body = html.encode(); self.send_response(200); self.send_header("Content-Type", "text/html; charset=utf-8")
            else:
                body = b"not found"; self.send_response(404)
            self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)
    ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()
