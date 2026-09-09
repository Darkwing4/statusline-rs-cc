(ns statusline-builder.colour-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.colour :as colour]
            [statusline-builder.preview :as preview]))

(def ^:private named {:kind :named :code 32})

(deftest context-gradient-matches-the-runtime-stops-and-truncation
  (is (= [147 153 178] (colour/gradient-rgb colour/context-gradient-stops -1 :truncate)))
  (is (= [147 153 178] (colour/gradient-rgb colour/context-gradient-stops 0 :truncate)))
  (is (= [198 189 176] (colour/gradient-rgb colour/context-gradient-stops 10 :truncate)))
  (is (= [249 226 175] (colour/gradient-rgb colour/context-gradient-stops 20 :truncate)))
  (is (= [246 182 171] (colour/gradient-rgb colour/context-gradient-stops 25 :truncate)))
  (is (= [243 139 168] (colour/gradient-rgb colour/context-gradient-stops 30 :truncate)))
  (is (= [243 139 168] (colour/gradient-rgb colour/context-gradient-stops 100 :truncate)))
  (is (= "#f6b6ab" (colour/context-css colour/gradient 25))))

(deftest cache-gradient-matches-active-and-cold-runtime-modes
  (is (= [249 202 155] (colour/gradient-rgb colour/cache-ttl-gradient-stops 80 :truncate)))
  (is (= [246 159 151] (colour/gradient-rgb colour/cache-ttl-gradient-stops 95 :truncate)))
  (is (= [249 202 155] (colour/gradient-rgb colour/cache-cold-gradient-stops 57.5 :truncate)))
  (is (= [246 159 151] (colour/gradient-rgb colour/cache-cold-gradient-stops 87.5 :truncate)))
  (is (= "#f9ca9b" (colour/cache-ttl-css colour/gradient {:cold false :percentage 80})))
  (is (= "#f69f97" (colour/cache-ttl-css colour/gradient {:cold true :percentage 87.5}))))

(deftest cache-scenario-derives-text-and-burned-percentage-from-runtime-values
  (let [active (preview/cache-ttl {:cache-ttl-seconds 300 :cache-remaining-seconds 272 :context 42})
        clean (preview/cache-ttl {:cache-ttl-seconds 3600 :cache-remaining-seconds 484 :context 12})
        cold (preview/cache-ttl {:cache-ttl-seconds 300 :cache-remaining-seconds 0 :context 88})]
    (is (= {:cold false :text "4m32s"} (dissoc active :percentage)))
    (is (< (abs (- (* (/ 28 300) 100) (:percentage active))) 1e-9))
    (is (= {:cold false :text "8m04s"} (dissoc clean :percentage)))
    (is (< (abs (- (* (/ 3116 3600) 100) (:percentage clean))) 1e-9))
    (is (= {:cold true :percentage 88 :text "cold"} cold))
    (is (= "#a0a2b1" (colour/cache-ttl-css colour/gradient active)))
    (is (= "#f9bb8d" (colour/cache-ttl-css colour/gradient clean)))))

(deftest rate-limit-gradient-uses-runtime-fallbacks-for-non-rgb-colours
  (is (= [166 227 161] (colour/colour->rgb named [166 227 161])))
  (is (= "#d0e3a8" (colour/interpolate-stops (colour/colour->rgb named [166 227 161])
                                             (colour/colour->rgb named [249 226 175])
                                             (colour/colour->rgb named [243 139 168])
                                             25
                                             50))))
