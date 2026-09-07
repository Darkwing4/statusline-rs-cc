(ns statusline-builder.colour-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.colour :as colour]
            [statusline-builder.preview :as preview]))

(def ^:private named {:kind :named :code 32})

(deftest context-gradient-matches-the-runtime-stops-and-truncation
  (is (= [150 150 150] (colour/gradient-rgb colour/context-gradient-stops -1 :truncate)))
  (is (= [150 150 150] (colour/gradient-rgb colour/context-gradient-stops 0 :truncate)))
  (is (= [165 157 125] (colour/gradient-rgb colour/context-gradient-stops 10 :truncate)))
  (is (= [180 165 100] (colour/gradient-rgb colour/context-gradient-stops 20 :truncate)))
  (is (= [200 112 80] (colour/gradient-rgb colour/context-gradient-stops 25 :truncate)))
  (is (= [220 60 60] (colour/gradient-rgb colour/context-gradient-stops 30 :truncate)))
  (is (= [220 60 60] (colour/gradient-rgb colour/context-gradient-stops 100 :truncate)))
  (is (= "#c87050" (colour/context-css colour/gradient 25))))

(deftest cache-gradient-matches-active-and-cold-runtime-modes
  (is (= [215 140 70] (colour/gradient-rgb colour/cache-ttl-gradient-stops 80 :truncate)))
  (is (= [242 75 55] (colour/gradient-rgb colour/cache-ttl-gradient-stops 95 :truncate)))
  (is (= [215 140 70] (colour/gradient-rgb colour/cache-cold-gradient-stops 57.5 :truncate)))
  (is (= [242 75 55] (colour/gradient-rgb colour/cache-cold-gradient-stops 87.5 :truncate)))
  (is (= "#d78c46" (colour/cache-ttl-css colour/gradient {:cold false :percentage 80})))
  (is (= "#f24b37" (colour/cache-ttl-css colour/gradient {:cold true :percentage 87.5}))))

(deftest cache-scenario-derives-text-and-burned-percentage-from-runtime-values
  (let [active (preview/cache-ttl {:cache-ttl-seconds 300 :cache-remaining-seconds 272 :context 42})
        clean (preview/cache-ttl {:cache-ttl-seconds 3600 :cache-remaining-seconds 484 :context 12})
        cold (preview/cache-ttl {:cache-ttl-seconds 300 :cache-remaining-seconds 0 :context 88})]
    (is (= {:cold false :text "4m32s"} (dissoc active :percentage)))
    (is (< (abs (- (* (/ 28 300) 100) (:percentage active))) 1e-9))
    (is (= {:cold false :text "8m04s"} (dissoc clean :percentage)))
    (is (< (abs (- (* (/ 3116 3600) 100) (:percentage clean))) 1e-9))
    (is (= {:cold true :percentage 88 :text "cold"} cold))
    (is (= "#828072" (colour/cache-ttl-css colour/gradient active)))
    (is (= "#e0713f" (colour/cache-ttl-css colour/gradient clean)))))

(deftest rate-limit-gradient-uses-runtime-fallbacks-for-non-rgb-colours
  (is (= [60 200 60] (colour/colour->rgb named [60 200 60])))
  (is (= "#8cc832" (colour/interpolate-stops (colour/colour->rgb named [60 200 60])
                                             (colour/colour->rgb named [220 200 40])
                                             (colour/colour->rgb named [220 60 60])
                                             25
                                             50))))
