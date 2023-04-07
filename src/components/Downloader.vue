<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";

const state = ref("");
const uuid = ref("1960050");
const cookie = ref("_ga_75BBYNYN9J=GS1.1.1680857407.49.1.1680858856.0.0.0; _fbp=fb.1.1673501034604.1950696989; __cf_bm=dKKy3vd6I7cgQ0hgZe_IwggJcmOOnxn6ERkSX7xjUpM-1680858782-0-AUi3gQdJCkf0Q5qepI05M/462X3A3mNOc3jWmrACHn6/xCTsfLD3uS2XTtjzNohmDRuthevUVItR529MvkcHuYXuwcay1IjtHI1874ILxOsa3yYI0Cq+CmFFeU/WX5Gmw5peWUOGKQdB/O/slE9F7O5+HPd76iTEs2LtteEcHrRKcXD297aHPGPbqOeBp873qQ==; _ga=GA1.1.625693455.1673501032; PHPSESSID=33192056_jFm1Tvb7uPEaBHSAOlMw7JZauSY6bO8a; b_type=0; privacy_policy_notification=0; _im_vid=01GPJ6ZA2SCTVEMRRZW4JAZ1K6; c_type=21; QSI_S_ZN_5hF4My7Ad6VNNAi=v:0:0; p_b_type=1; tag_view_ranking=0xsDLqCEW6~KMpT0re7Sq~SIpXPnQ53M~rMbyd5uAhj~Lt-oEicbBr~_EOd7bsGyl~RTJMXD26Ak~UgLGWGysi-~AoKfsFwwdu~Vt3Tl4Tkoa~BSlt10mdnm~Tcn3gevBtQ~HHxwTpn5dx~jH0uD88V6F~6cIiIlKj-K~wm006gFVAz~CAhAmfRBQs~vP6kTD-0Xd~Bd2L9ZBE8q~faHcYIP1U0~ziiAzr_h04~Ie2c51_4Sp~l6cxseQIBN~O8u4lwX_cT~2VnbKugRTI~RiZgkjd5Cv~26-Sd3V3Py~D9BseuUB5Z~lW3PDRuOC9~CiSfl_AE0h~e2yEFDVXjZ~YI8LmI20qW~EUwzYuPRbU~azESOjmQSV~7dpqkQl8TH~oDcj90OVdf~GNcgbuT3T-~ETjPkL0e6r~OT4SuGenFI~kP7msdIeEU~RXk9hi7kn_~QOc7RQXB8U~LfyX5eCTtL~lH5YZxnbfC~aKhT3n4RHZ~kGYw4gQ11Z~tgP8r-gOe_~KN7uxuR89w~jm40SVtdHx~K8esoIs2eW~ti_E1boC1J~zIv0cf5VVk~mLrrjwTHBm~alQb7gJxOf~pnCQRVigpy~J4-uQ7g8Dw~G_vM51w8ml~HMU_P-aYJG~PgQgNyh9aH~zyKU3Q5L4C~PwDMGzD6xn~BtXd1-LPRH~kwxbx_VxB2~1NmdBLOfGO~os5VB0oZyX~o7hvUrSGDN~2QTW_H5tVX~eVxus64GZU~H9o4sUN8F1~pzzjRSV6ZO~Qri62qoiFF~fg8EOt4owo~7fCik7KLYi~ION4v6ZHqO~9Gbahmahac~9wN-K8_crj~pzZvureUki~JJ4D2-VDRE~iFcW6hPGPU~EGefOqA6KB~aOGQhsapNP~LEmyJ-RN72~j6sKkcbNBV~MUQoS0sfqG~0J097IdBNd~zb1N_JZSZu~BA8VCLPrP0~zd0kMkvoqd~Cpw-lbF1eB~1HSjrqSB3U~_pwIgrV8TB~uu4WDPyt4x~Z4hQZu-rU-~FPR_bzUx-I~-RR2Rsko5M~ZXFMxANDG_~JXmGXDx4tL~eInvgwdvwj~gUIg7nrQgl~02lg2Bq_mf; __utma=235335808.625693455.1673501032.1676024046.1676024046.1; __utmv=235335808.|3=plan=normal=1^6=user_id=33192056=1^11=lang=zh=1; __utmz=235335808.1676024046.1.1.utmcsr=(direct)|utmccn=(direct)|utmcmd=(none); adr_id=Q1VxuKbvVTCs14KBezq5vzQebRnn4adNbdw2JETMs2XDegoP; _ts_yjad=1674656290200; _ga_1Q464JT0V2=GS1.1.1674656289.1.0.1674656289.0.0.0; _ga_MZ1NL4PHH0=GS1.1.1674503773.2.0.1674503775.0.0.0; first_visit_datetime=2023-01-23+20%3A29%3A05; _gcl_au=1.1.27740088.1674473256; pt_60er4xix=uid=Wp3sTmzAFRxIqQLKMDIJLw&nid=0&vid=Z/PFHXF9ngOrUL7ninxXmQ&vn=2&pvn=1&sact=1674465855673&to_flag=0&pl=JWYJn-k8h/7K2Bjkw-IBJA*pt*1674465849805; a_type=1; _im_uid.3929=b.c66866c9851b7f8e; login_ever=yes; privacy_policy_agreement=5; p_ab_d_id=1824640068; p_ab_id=0; p_ab_id_2=3; first_visit_datetime_pc=2022-10-01+04%3A16%3A48; yuid_b=g3FAREA");
const path = ref("./Downloads");
const proxy = ref("http://127.0.0.1:7890");

async function go() {
  if (uuid.value == '' || cookie.value == '') {
    state.value = "missing param"
    return
  };
  state.value = "Downloading";
  await invoke("go", { uuid: uuid.value, cookie: cookie.value, path: path.value, proxy: proxy.value });
  state.value = "Finish";
}
</script>

<template>
  <div class="card">
    <label>Artist ID:</label>
    <input id="id-input" v-model="uuid" placeholder="Enter a pixiv artist id..." />
    <br>
    <label>Cookie:</label>
    <input id="cookie-input" v-model="cookie" placeholder="Enter your cookie" />
    <br>
    <label>Save path:</label>
    <input id="path-input" v-model="path" placeholder="Enter your path" />
    <br>
    <label>Proxy:</label>
    <input id="proxy-input" v-model="proxy" placeholder="Enter your proxy" />
    <br>
    <button type="button" @click="go()">Go</button>
    <p>{{ state }}</p>
  </div>
</template>

<style>
.card {
  display: flex;
  flex-direction: column;
  align-items: center;
}
</style>